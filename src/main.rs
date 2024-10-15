use zero2prod::{
    configuration::get_configuration, startup::Application, telemetry::*,
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_subscriber(get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    ));

    let config = get_configuration().expect("Failed to read configuration");

    let application = Application::build(&config)?;
    application.run_until_stopped().await?;
    Ok(())
}
