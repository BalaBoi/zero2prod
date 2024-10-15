use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server
}

impl Application {
    pub fn build(settings: &Settings) -> Result<Self, std::io::Error> {
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
            settings.email_client_settings.timeout()
        );
        
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;
        
        Ok(Self {
            port,
            server,
        })
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

pub fn run(
    listener: TcpListener,
    database_connection: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(database_connection);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
