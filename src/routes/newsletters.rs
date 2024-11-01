use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    domain::SubscriberEmail,
    email_client::EmailClient,
};
use actix_web::{
    http::header::{self, HeaderMap, HeaderValue},
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use base64::Engine;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::{PgPool, Row};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            Self::AuthError(_) => {
                let mut response = HttpResponse::Unauthorized();
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response.insert_header((header::WWW_AUTHENTICATE, header_value));
                response.finish()
            }
            Self::UnexpectedError(_) => HttpResponse::InternalServerError().finish(),
        }
    }
}

#[tracing::instrument(
    name = "Publish newsletters to all confirmed subscribers",
    skip(pool, email_client, body),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    body: web::Json<BodyData>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|err| match err {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(err.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(err.into()),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(ConfirmedSubscriber { email }) => {
                email_client
                    .send_email(&email, &body.title, &body.content.html, &body.content.text)
                    .await
                    .with_context(|| format!("Failed to send newsletter to {}", email))?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Subscriber with status set to confirmed failed in being validated"
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
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
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF-8 string")?;

    let b64_usrname_pwd = header_value
        .strip_prefix("Basic ")
        .context("The Authorization scheme was not 'Basic'")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64_usrname_pwd)
        .context("Failed to base64-decode 'Basic' credentials")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid utf-8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth"))?;

    Ok(Credentials {
        username,
        password: SecretString::new(password.into()),
    })
}
