use crate::{
    credentials::{normalize_email, validate_password},
    error::AppError,
    models::user::User,
    state::AppState,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub sid: Uuid,
    pub jti: Uuid,
    pub exp: u64,
    pub iss: String,
    pub aud: String,
}
#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub refresh_token: String,
    pub refresh_expires_in: u64,
    pub user: User,
}

pub fn now() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .map_err(|_| AppError::internal("Invalid system clock"))
}
fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
fn refresh_credential() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub async fn hash_password(state: &AppState, password: String) -> Result<String, AppError> {
    validate_password(&password).map_err(AppError::bad_request)?;
    let permit = state
        .password_tasks
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::ServiceUnavailable)?;
    tokio::task::spawn_blocking(move || {
        // Request cancellation must not release capacity while CPU work is still running.
        let _permit = permit;
        Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|value| value.to_string())
            .map_err(|_| AppError::internal("Password hashing failed"))
    })
    .await
    .map_err(|_| AppError::internal("Password hashing failed"))?
}

pub fn validate_name(name: Option<String>) -> Result<Option<String>, AppError> {
    name.map(|name| {
        let name = name.trim();
        if !(2..=100).contains(&name.chars().count()) {
            return Err(AppError::bad_request(
                "Name must contain 2 to 100 characters",
            ));
        }
        Ok(name.to_owned())
    })
    .transpose()
}

pub async fn register(state: &AppState, payload: Credentials) -> Result<TokenResponse, AppError> {
    let email = normalize_email(&payload.email).map_err(AppError::bad_request)?;
    let name = validate_name(payload.name)?;
    let hash = hash_password(state, payload.password).await?;
    let user: User = sqlx::query_as("INSERT INTO users (id, email, password_hash, name) VALUES ($1,$2,$3,$4) RETURNING id,email,name,role,disabled")
        .bind(Uuid::new_v4()).bind(email).bind(hash).bind(name).fetch_one(&state.db).await
        .map_err(|error| if error.as_database_error().is_some_and(|e| e.is_unique_violation()) { AppError::Conflict } else { error.into() })?;
    issue_session(state, user).await
}

pub async fn login(state: &AppState, payload: Credentials) -> Result<TokenResponse, AppError> {
    let email = normalize_email(&payload.email).map_err(|_| AppError::Unauthorized)?;
    if payload.password.len() > 128 {
        return Err(AppError::Unauthorized);
    }
    let permit = state
        .password_tasks
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::ServiceUnavailable)?;
    let record: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id,password_hash FROM users WHERE email=$1 AND NOT disabled")
            .bind(email)
            .fetch_optional(&state.db)
            .await?;
    let id = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        // A valid dummy hash avoids bypassing Argon2 for unknown accounts.
        let (id, hash) = match record {
            Some((id, hash)) => (Some(id), hash),
            None => (
                None,
                Argon2::default()
                    .hash_password(b"unknown-account-dummy", &SaltString::generate(&mut OsRng))
                    .map_err(|_| AppError::internal("Password verification failed"))?
                    .to_string(),
            ),
        };
        let parsed = PasswordHash::new(&hash).map_err(|_| AppError::Unauthorized)?;
        Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed)
            .map_err(|_| AppError::Unauthorized)?;
        id.ok_or(AppError::Unauthorized)
    })
    .await
    .map_err(|_| AppError::internal("Password verification failed"))??;
    let user = sqlx::query_as(
        "SELECT id,email,name,role,disabled FROM users WHERE id=$1 AND NOT disabled",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;
    issue_session(state, user).await
}

fn token_response(
    state: &AppState,
    user: User,
    sid: Uuid,
    jti: Uuid,
    refresh_token: String,
) -> Result<TokenResponse, AppError> {
    let claims = Claims {
        sub: user.id,
        sid,
        jti,
        exp: now()? + state.auth.access_ttl,
        iss: state.auth.issuer.clone(),
        aud: state.auth.audience.clone(),
    };
    let access_token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::internal("Token creation failed"))?;
    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: state.auth.access_ttl,
        refresh_token,
        refresh_expires_in: state.auth.refresh_ttl,
        user,
    })
}

async fn issue_session(state: &AppState, user: User) -> Result<TokenResponse, AppError> {
    let sid = Uuid::new_v4();
    let jti = Uuid::new_v4();
    let refresh = refresh_credential();
    let expiry = (now()? + state.auth.refresh_ttl) as i64;
    let response = token_response(state, user.clone(), sid, jti, refresh.clone())?;
    sqlx::query("INSERT INTO auth_sessions (id,user_id,refresh_hash,refresh_expires_at,access_jti) VALUES ($1,$2,$3,$4,$5)")
        .bind(sid).bind(user.id).bind(hash_token(&refresh)).bind(expiry).bind(jti).execute(&state.db).await?;
    Ok(response)
}

pub async fn refresh(state: &AppState, token: &str) -> Result<TokenResponse, AppError> {
    if token.len() != 64 {
        return Err(AppError::Unauthorized);
    }
    let next = refresh_credential();
    let jti = Uuid::new_v4();
    let mut tx = state.db.begin().await?;
    // UPDATE locks and consumes the matching credential atomically across server instances.
    let session: Option<(Uuid, Uuid)> = sqlx::query_as(
        "UPDATE auth_sessions SET refresh_hash=$2, access_jti=$3, refresh_expires_at=$4 \
         WHERE refresh_hash=$1 AND NOT revoked AND refresh_expires_at>$5 RETURNING id,user_id",
    )
    .bind(hash_token(token))
    .bind(hash_token(&next))
    .bind(jti)
    .bind((now()? + state.auth.refresh_ttl) as i64)
    .bind(now()? as i64)
    .fetch_optional(&mut *tx)
    .await?;
    let (sid, id) = session.ok_or(AppError::Unauthorized)?;
    let user: User = sqlx::query_as(
        "SELECT id,email,name,role,disabled FROM users WHERE id=$1 AND NOT disabled",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let response = token_response(state, user, sid, jti, next)?;
    tx.commit().await?;
    Ok(response)
}

pub async fn logout(state: &AppState, claims: &Claims) -> Result<(), AppError> {
    sqlx::query("UPDATE auth_sessions SET revoked=TRUE WHERE id=$1 AND user_id=$2")
        .bind(claims.sid)
        .bind(claims.sub)
        .execute(&state.db)
        .await?;
    Ok(())
}
