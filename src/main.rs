use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::{configuration::get_configuration, startup::run, telemetry::*};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_subscriber(get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    ));
    let config = get_configuration().expect("Failed to read configuration");
    let address = format!(
        "{}:{}",
        config.application_settings.host, config.application_settings.port
    );
    let listener = TcpListener::bind(address)?;
    let database_pool =
        PgPool::connect_lazy(&config.database_settings.connection_string().expose_secret())
            .expect("Couldn't get database connection");
    run(listener, database_pool)?.await
}
