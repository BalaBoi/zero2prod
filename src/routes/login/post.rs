use actix_web::{error::InternalError, http::header::LOCATION, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::SecretString;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, AuthError, Credentials};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(
    name = "User login",
    skip(pool, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.username.clone(),
        password: form.password.clone(),
    };
    tracing::Span::current().record("username", &credentials.username);
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", user_id.to_string());
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(error) => {
            let error = match error {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(error.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(error.into()),
            };
            FlashMessage::error(error.to_string()).send();
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish();
            Err(InternalError::from_response(error, response))
        }
    }
}
