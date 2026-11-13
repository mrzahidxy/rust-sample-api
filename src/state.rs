use crate::config::Config;
use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(Clone)]
pub struct AppState {
    pub http: crate::config::HttpConfig,
    pub rate_limiter: std::sync::Arc<crate::middleware::http::RateLimiter>,
    pub auth: crate::config::AuthConfig,
    pub db: PgPool,
    /// Shared across router clones; bound memory used by expensive password operations.
    pub password_tasks: std::sync::Arc<tokio::sync::Semaphore>,
    pub jwt_secret: std::sync::Arc<str>,
}

impl AppState {
    pub async fn connect(config: &Config) -> Result<Self, sqlx::Error> {
        let db = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(config.acquire_timeout)
            .connect(&config.database_url)
            .await?;
        Ok(Self {
            http: config.http.clone(),
            rate_limiter: Default::default(),
            auth: config.auth.clone(),
            db,
            password_tasks: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
            jwt_secret: config.jwt_secret.clone().into(),
        })
    }
}

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
