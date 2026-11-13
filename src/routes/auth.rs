use crate::{
    error::AppError,
    models::user::User,
    services::auth::{self, Claims, Credentials, TokenResponse},
    state::AppState,
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

pub fn routes(state: &AppState) -> Router<AppState> {
    let protected = Router::new()
        .route("/me", get(me))
        .route("/logout", post(logout))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::require_auth,
        ));
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .merge(protected)
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
}
async fn register(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<(StatusCode, Json<TokenResponse>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(auth::register(&state, payload).await?),
    ))
}
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<Json<TokenResponse>, AppError> {
    Ok(Json(auth::login(&state, payload).await?))
}
#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}
async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    Ok(Json(auth::refresh(&state, &payload.refresh_token).await?))
}
async fn me(Extension(user): Extension<User>) -> Json<User> {
    Json(user)
}
async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode, AppError> {
    auth::logout(&state, &claims).await?;
    Ok(StatusCode::NO_CONTENT)
}
