use sqlx::PgPool;
use std::net::TcpListener;
use zer02prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = get_configuration().expect("Failed to read configuration");
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)?;
    let database_pool = PgPool::connect(&config.database_settings.connection_string())
        .await
        .expect("Couldn't get database connection");
    run(listener, database_pool)?.await
}
