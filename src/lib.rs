pub mod auth;
pub mod config;
pub mod credentials;
pub mod docs;
pub mod error;
pub mod middleware;
pub mod models;
pub mod pagination;

pub mod routes;
pub mod services;
pub mod state;

use crate::{error::AppError, state::AppState};
use axum::{routing::get, Json, Router};
use tower_http::trace::TraceLayer;

/// Compose the application independently of its TCP listener for reuse and tests.
pub fn app(state: AppState) -> Router {
    let origins: Vec<axum::http::HeaderValue> = state
        .http
        .cors_origins
        .iter()
        .map(|origin| origin.parse().expect("validated CORS origin"))
        .collect();
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
        .expose_headers([axum::http::HeaderName::from_static("x-request-id")]);
    Router::new()
        .merge(routes::health::health_routes())
        .merge(docs::routes())
        .nest("/auth", routes::auth::routes(&state))
        .nest("/api/auth", routes::auth::routes(&state))
        .nest("/api", routes::projects::routes()
            .merge(routes::users::routes())
            .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth::require_auth)))
        .route("/ready", get(readiness))
        .fallback(|| async { AppError::not_found("Route not found") })
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<axum::body::Body>| {
            tracing::info_span!("http_request", method = %request.method(), path = %request.uri().path(),
                request_id = ?request.headers().get("x-request-id"))
        }))
        .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::http::context))
        .layer(cors)
        .with_state(state)
}

async fn readiness(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "status": "ready" })))
}
