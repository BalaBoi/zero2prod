use std::net::TcpListener;
use sqlx::{Connection, PgConnection};
use zer02prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = get_configuration().expect("Failed to read configuration");
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)?;
    let database_connection = PgConnection::connect(
            &config.database_settings.connection_string()
        )
        .await
        .expect("Couldn't get database connection");
    run(listener, database_connection)?.await
}
