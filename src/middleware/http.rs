use crate::state::AppState;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;
#[derive(Default)]
pub struct RateLimiter {
    windows: Mutex<HashMap<(IpAddr, bool), (Instant, u32)>>,
}
impl RateLimiter {
    fn allow(&self, ip: IpAddr, auth: bool) -> bool {
        let Ok(mut windows) = self.windows.lock() else {
            return false;
        };
        let now = Instant::now();
        if windows.len() >= 10000 {
            windows.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(900));
        }
        let key = (ip, auth);
        if windows.len() >= 10000 && !windows.contains_key(&key) {
            return false;
        }
        let (start, count) = windows.entry(key).or_insert((now, 0));
        if now.duration_since(*start) >= Duration::from_secs(900) {
            *start = now;
            *count = 0;
        }
        let max = if auth { 30 } else { 100 };
        if *count >= max {
            return false;
        }
        *count += 1;
        true
    }
}
pub async fn context(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    // Generate server-controlled IDs; never trust arbitrary caller values in logs.
    let id = Uuid::new_v4().to_string();
    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&id).expect("UUID header"),
    );
    let path = request.uri().path();
    let auth = path.starts_with("/api/auth/") || path.starts_with("/auth/");
    let health = matches!(path, "/health" | "/ready");
    let allowed = health
        || !state.http.rate_limit_enabled
        || request
            .extensions()
            .get::<ConnectInfo<std::net::SocketAddr>>()
            .is_none_or(|ConnectInfo(address)| state.rate_limiter.allow(address.ip(), auth));
    let mut response = if allowed {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error":"Too many requests; retry later"})),
        )
            .into_response()
    };
    let limited = response.status() == StatusCode::TOO_MANY_REQUESTS;
    let headers = response.headers_mut();
    headers.insert(
        "x-request-id",
        HeaderValue::from_str(&id).expect("UUID header"),
    );
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    if limited {
        headers.insert("retry-after", HeaderValue::from_static("900"));
    }
    response
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auth_budget_is_separate_and_bounded() {
        let limiter = RateLimiter::default();
        let ip = "127.0.0.1".parse().unwrap();
        for _ in 0..30 {
            assert!(limiter.allow(ip, true));
        }
        assert!(!limiter.allow(ip, true));
        assert!(limiter.allow(ip, false));
    }
}
