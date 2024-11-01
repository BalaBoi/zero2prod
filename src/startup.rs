use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    confirm, health_check, home, login, login_form, publish_newsletter, subscribe,
};
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(settings: &Settings) -> Result<Self, anyhow::Error> {
        let address = format!(
            "{}:{}",
            settings.application_settings.host, settings.application_settings.port
        );
        let listener = TcpListener::bind(address)?;

        let connection_pool = get_connection_pool(&settings.database_settings);

        let sender_email = settings
            .email_client_settings
            .sender()
            .expect("Invalid sender email address config value");
        let email_client = EmailClient::new(
            &settings.email_client_settings.base_url,
            sender_email,
            &settings.email_client_settings.authorization_token,
            settings.email_client_settings.timeout(),
        );

        let base_url = settings.application_settings.base_url.as_str();

        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client, base_url, &settings.redis_uri).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(db_settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(db_settings.with_db())
}

pub struct ApplicationBaseUrl(pub String);

pub async fn run(
    listener: TcpListener,
    database_connection: PgPool,
    email_client: EmailClient,
    base_url: &str,
    redis_uri: &SecretString,
) -> Result<Server, anyhow::Error> {
    let connection = web::Data::new(database_connection);
    let email_client = web::Data::new(email_client);
    let app_base_url = web::Data::new(ApplicationBaseUrl(base_url.to_owned()));
    let flash_message_key = Key::generate();
    let redis_secret_key = Key::generate();
    let message_store = CookieMessageStore::builder(flash_message_key).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(SessionMiddleware::new(redis_store.clone(), redis_secret_key.clone()))
            .wrap(message_framework.clone())
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(app_base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
