use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionConfirmationError {
    #[error("No subscriber pending confirmation for the provided confirmation token")]
    UnauthorizedError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscriptionConfirmationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::UnauthorizedError => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, SubscriptionConfirmationError> {
    let id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await
        .context("Couldn't get subscription id from database for confirmation token")?;
    match id {
        Some(subscriber_id) => {
            confirm_suscriber(&pool, subscriber_id)
                .await
                .context("Couldn't set subscriber as confirmed in the database")?;
        }
        None => return Err(SubscriptionConfirmationError::UnauthorizedError),
    }
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, subscription_token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_token
        WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
async fn confirm_suscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
