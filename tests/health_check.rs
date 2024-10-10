use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Executor, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup,
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
    address: String,
    db_pool: PgPool,
}

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn valid_subscribe_returns_200() {
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(&format!("{}/subscriptions", test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn missing_data_subscribe_returns_400() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut db_config = get_configuration()
        .expect("Couldn't read configuration file")
        .database_settings;
    db_config.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_database(&db_config).await;
    let server = startup::run(listener, db_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    PgPool::connect(&config.connection_string_without_db().expose_secret())
        .await
        .expect("Failed to connect to postgres")
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Couldn't create a new test database");

    let connection_pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed connect to postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Couldn't run migrations on test db");

    connection_pool
}
