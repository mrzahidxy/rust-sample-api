use crate::{
    error::AppError,
    models::user::User,
    services::auth::{now, Claims},
    state::AppState,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[state.auth.issuer.as_str()]);
    validation.set_audience(&[state.auth.audience.as_str()]);
    validation.leeway = 0;
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;
    // Fetch current role. Disabled/deleted accounts and rotated/revoked sessions stop working immediately.
    let user: User = sqlx::query_as(
        "SELECT u.id,u.email,u.name,u.role,u.disabled FROM users u JOIN auth_sessions s ON s.user_id=u.id \
         WHERE u.id=$1 AND s.id=$2 AND s.access_jti=$3 AND NOT u.disabled AND NOT s.revoked AND s.refresh_expires_at>$4")
        .bind(claims.sub).bind(claims.sid).bind(claims.jti).bind(now()? as i64)
        .fetch_optional(&state.db).await?.ok_or(AppError::Unauthorized)?;
    request.extensions_mut().insert(claims);
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}
