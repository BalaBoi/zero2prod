use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;
use zer02prod::{configuration::get_configuration, startup::run, telemetry::*};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_subscriber(get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    ));
    let config = get_configuration().expect("Failed to read configuration");
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)?;
    let database_pool = PgPool::connect(&config.database_settings.connection_string().expose_secret())
        .await
        .expect("Couldn't get database connection");
    run(listener, database_pool)?.await
}
