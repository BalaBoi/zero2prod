use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{configuration::{get_configuration, DatabaseSettings}, startup::{get_connection_pool, Application}, telemetry::{get_subscriber, init_subscriber}};

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
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let settings = {
        let mut settings = get_configuration().expect("Failed to read configuration");
        settings.database_settings.database_name = Uuid::new_v4().into();
        settings.application_settings.port = 0;
        settings
    };

    configure_database(&settings.database_settings).await;

    let application = Application::build(&settings).expect("Failed to build application");
    let address = format!("http://127.0.0.1:{}", application.port());
    std::mem::drop(tokio::spawn(application.run_until_stopped()));

    TestApp {
        address,
        db_pool: get_connection_pool(&settings.database_settings)
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
