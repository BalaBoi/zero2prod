use anyhow::Context;
use argon2::{
    password_hash::SaltString, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_trace;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    let (user_id, expected_hash) = get_stored_credentials(&credentials.username, pool)
        .await
        .map_err(AuthError::UnexpectedError)?
        .ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username")))?;

    spawn_blocking_with_trace(move || validate_password(&credentials.password, &expected_hash))
        .await
        .context("Failed to join spawn blocking task")??;

    Ok(user_id)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials")?
    .map(|row| {
        (
            row.user_id,
            SecretString::new(row.password_hash.into_boxed_str()),
        )
    });
    Ok(row)
}

#[tracing::instrument(
    name = "Validating password with expected hash",
    skip(password, expected_hash)
)]
fn validate_password(
    password: &SecretString,
    expected_hash: &SecretString,
) -> Result<(), AuthError> {
    let expected_hash = PasswordHash::new(expected_hash.expose_secret())
        .context("Failed to parse password_hash (from users table) in PHC format")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: Uuid,
    password: SecretString,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_trace(move || compute_password_hash(password))
        .await?
        .context("failed to hash password")?;
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database")?;
    Ok(())
}

fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(SecretString::new(password_hash.into_boxed_str()))
}
