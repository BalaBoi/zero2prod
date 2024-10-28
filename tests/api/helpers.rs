use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    match std::env::var("TEST_LOG") {
        Ok(_) => {
            let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
            init_subscriber(subscriber);
        }
        Err(_) => {
            let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::sink);
            init_subscriber(subscriber);
        }
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscription<B: Into<reqwest::Body>>(&self, body: B) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn post_newsletters(&self, body: &serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/newsletters", self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub struct TestUser {
    user_id: Uuid,
    username: String,
    password: String,
}

impl TestUser {
    async fn generate(pool: &PgPool) -> Self {
        let this = Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        };
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(this.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)
            "#,
            &this.user_id,
            &this.username,
            &password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to create test users");
        this
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let settings = {
        let mut settings = get_configuration().expect("Failed to read configuration");
        settings.database_settings.database_name = Uuid::new_v4().into();
        settings.application_settings.port = 0;
        settings.email_client_settings.base_url = email_server.uri();
        settings
    };

    configure_database(&settings.database_settings).await;

    let application = Application::build(&settings).expect("Failed to build application");
    let application_port = application.port();
    std::mem::drop(tokio::spawn(application.run_until_stopped()));

    let pool = get_connection_pool(&settings.database_settings);
    let test_user = TestUser::generate(&pool).await;
    TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        db_pool: pool,
        email_server,
        port: application_port,
        test_user,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Couldn't create a new test database");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed connect to postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Couldn't run migrations on test db");

    connection_pool
}

pub fn get_confirmation_links(email_request: &wiremock::Request, port: u16) -> ConfirmationLinks {
    let get_link = |s: &str| {
        let links = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 1);
        let raw_link = links[0].as_str().to_owned();
        let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
        assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
        confirmation_link.set_port(Some(port)).unwrap();
        confirmation_link
    };
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let html = get_link(body["content"][0]["value"].as_str().unwrap());
    let plain_text = get_link(body["content"][1]["value"].as_str().unwrap());

    ConfirmationLinks {
        html,
        text: plain_text,
    }
}
