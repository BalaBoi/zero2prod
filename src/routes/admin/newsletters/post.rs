use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Row};

use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct NewsletterForm {
    title: String,
    html: String,
    text: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber(SubscriberEmail);

#[tracing::instrument(
    name = "Publish newsletters to all confirmed subscribers",
    skip(pool, form, email_client, user_id),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<NewsletterForm>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let NewsletterForm {
        title,
        html,
        text,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let transaction = match try_processing(&pool, &idempotency_key, **user_id).await.map_err(e500)? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            success_message().send();
            return Ok(response);
        }
    };
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(ConfirmedSubscriber(email)) => {
                email_client
                    .send_email(&email, &title, &html, &text)
                    .await
                    .with_context(|| format!("Failed to send newsletter to {}", email))
                    .map_err(e500)?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    error.message = %error,
                    "Subscriber with status set to confirmed failed in being validated"
                )
            }
        };
    }
    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, **user_id, response).await.map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| match SubscriberEmail::parse(row.get("email")) {
        Ok(email) => Ok(ConfirmedSubscriber(email)),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}
