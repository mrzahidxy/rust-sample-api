use crate::state::AppState;
use axum::{extract::State, response::Html, routing::get, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/openapi.json",
            get(|State(state): State<AppState>| async move {
                let mut spec =
                    serde_json::from_str::<serde_json::Value>(include_str!("../openapi.json"))
                        .expect("valid embedded OpenAPI document");
                spec["info"]["title"] = state.http.app_name.into();
                spec["info"]["description"] = state.http.app_description.into();
                Json(spec)
            }),
        )
        .route(
            "/docs",
            get(|| async { Html(include_str!("../docs.html")) }),
        )
}
