use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

pub fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
