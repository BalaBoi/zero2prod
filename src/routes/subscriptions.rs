use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{SubscriberEmail, SubscriberName};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        name = %form.name,
        email = %form.email
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> impl Responder {
    let subscriber_name = match SubscriberName::parse(&form.name) {
        Ok(sub_name) => sub_name,
        Err(_) => return HttpResponse::BadRequest().await,
    };
    let subscriber_email = match SubscriberEmail::parse(&form.email) {
        Ok(sub_email) => sub_email,
        Err(_) => return HttpResponse::BadRequest().await,
    };
    match insert_subscriber(pool.get_ref(), &subscriber_email, &subscriber_name).await {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => HttpResponse::InternalServerError(),
    }.await
}

#[tracing::instrument(name = "Saving new subscriber details in the db", skip(pool, email, name))]
pub async fn insert_subscriber(pool: &PgPool, email: &SubscriberEmail, name: &SubscriberName) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        email.as_ref(),
        name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {}", err);
        err
    })?;
    Ok(())
}
