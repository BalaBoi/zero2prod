use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let sub_email = SubscriberEmail::parse(&value.email)?;
        let sub_name = SubscriberName::parse(&value.name)?;
        Ok(Self {
            email: sub_email,
            name: sub_name,
        })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        name = %form.name,
        email = %form.email
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire postgres connection from the pool")?;
    let new_subscriber = form
        .into_inner()
        .try_into()
        .map_err(SubscribeError::ValidationError)?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber into the database")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store generated subscription token into the database")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to commit new subscriber")?;
    send_confirmation_email(
        &email_client,
        &new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send confirmation email for the new subscriber")?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument{
    name = "Store subscription token in the database",
    skip(transaction, subscriber_id, subscription_token)
}]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_token (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the db",
    skip(transaction, new_subscriber)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&mut **transaction)
    .await?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send a confirmation email to the new subscriber",
    skip(email_client, new_subscriber, base_url, confirmation_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    confirmation_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, confirmation_token
    );

    let text = format!(
        "Welcome to Sanjay's newsletter!\n Confirmation link: {}",
        confirmation_link
    );

    let html_body = format!("Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.", confirmation_link);

    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &text)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
