use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::{
    configuration::get_configuration, email_client::EmailClient, startup::run, telemetry::*,
};

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

    let database_pool = PgPoolOptions::new().connect_lazy_with(config.database_settings.with_db());

    let sender_email = config
        .email_client_settings
        .sender()
        .expect("Invalid sender email address config value");
    let email_client = EmailClient::new(
        &config.email_client_settings.base_url,
        sender_email,
        &config.email_client_settings.authorization_token,
        config.email_client_settings.timeout()
    );

    run(listener, database_pool, email_client)?.await
}
