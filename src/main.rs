use tokio::task::JoinError;
use std::fmt::{Display, Debug};
use zero2prod::{configuration::get_configuration, issue_delivery_worker::run_worker_until_stopped, startup::Application, telemetry::*};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_subscriber(get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    ));

    let config = get_configuration().expect("Failed to read configuration");

    let application_task = tokio::spawn(Application::build(&config).await?.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(config.clone()));

    tokio::select! {
        out = application_task => {report_exit("API", out)},
        out = worker_task => {report_exit("Background worker", out)}
    };

    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl Debug + Display>, JoinError>
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        },
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        },
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "'{}' task failed to complete",
                task_name
            )
        }
    }
}
