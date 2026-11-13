#[derive(Clone)]
pub struct HttpConfig {
    pub app_name: String,
    pub app_description: String,
    pub cors_origins: Vec<String>,
    pub rate_limit_enabled: bool,
}
impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            app_name: "Rust Backend".into(),
            app_description: "Rust project management API".into(),
            cors_origins: vec![
                "http://localhost:3000".into(),
                "http://localhost:5173".into(),
            ],
            rate_limit_enabled: true,
        }
    }
}

#[derive(Clone)]
pub struct AuthConfig {
    pub issuer: String,
    pub audience: String,
    pub access_ttl: u64,
    pub refresh_ttl: u64,
}
impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            issuer: "rust-service".into(),
            audience: "rust-service-api".into(),
            access_ttl: 1800,
            refresh_ttl: 7 * 86400,
        }
    }
}

use std::{env, error::Error, net::SocketAddr, time::Duration};

#[derive(Clone)]
pub struct Config {
    pub http: HttpConfig,
    pub auth: AuthConfig,
    pub address: SocketAddr,
    pub database_url: String,
    pub jwt_secret: String,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

impl Config {
    /// Load configuration after the caller has optionally loaded a .env file.
    pub fn from_env() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
        if database_url.trim().is_empty() {
            return Err("DATABASE_URL must not be empty".into());
        }
        let jwt_secret = env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;
        if jwt_secret.len() < 32 || jwt_secret.starts_with("replace-") {
            return Err("JWT_SECRET must be a random secret of at least 32 characters".into());
        }
        let address = env::var("APP_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:3000".into())
            .parse()?;
        let max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".into())
            .parse()?;
        if max_connections == 0 {
            return Err("DATABASE_MAX_CONNECTIONS must be greater than zero".into());
        }
        let timeout_seconds: u64 = env::var("DATABASE_ACQUIRE_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "5".into())
            .parse()?;
        if timeout_seconds == 0 {
            return Err("DATABASE_ACQUIRE_TIMEOUT_SECONDS must be greater than zero".into());
        }
        let defaults = AuthConfig::default();
        let auth = AuthConfig {
            issuer: env::var("JWT_ISSUER").unwrap_or(defaults.issuer),
            audience: env::var("JWT_AUDIENCE").unwrap_or(defaults.audience),
            access_ttl: env::var("ACCESS_TOKEN_TTL_SECONDS")
                .unwrap_or_else(|_| "1800".into())
                .parse()?,
            refresh_ttl: env::var("REFRESH_TOKEN_TTL_SECONDS")
                .unwrap_or_else(|_| "604800".into())
                .parse()?,
        };
        if auth.issuer.trim().is_empty()
            || auth.audience.trim().is_empty()
            || !(60..=86400).contains(&auth.access_ttl)
            || !(300..=31536000).contains(&auth.refresh_ttl)
        {
            return Err("Invalid JWT issuer, audience, or token TTL".into());
        }
        let defaults = HttpConfig::default();
        let rate_limit_enabled = match env::var("APP_RATE_LIMIT_ENABLED") {
            Ok(value) => value.parse()?,
            Err(env::VarError::NotPresent) => defaults.rate_limit_enabled,
            Err(error) => return Err(error.into()),
        };
        let http = HttpConfig {
            app_name: env::var("APP_NAME").unwrap_or(defaults.app_name),
            app_description: env::var("APP_DESCRIPTION").unwrap_or(defaults.app_description),
            cors_origins: env::var("CORS_ORIGINS")
                .map(|value| {
                    value
                        .split(',')
                        .map(|v| v.trim().to_owned())
                        .filter(|v| !v.is_empty())
                        .collect()
                })
                .unwrap_or(defaults.cors_origins),
            rate_limit_enabled,
        };
        for origin in &http.cors_origins {
            let uri: axum::http::Uri = origin.parse()?;
            if !matches!(uri.scheme_str(), Some("http" | "https"))
                || uri.authority().is_none()
                || uri.path() != "/"
                || uri.query().is_some()
            {
                return Err("CORS_ORIGINS must contain explicit HTTP(S) origins".into());
            }
            let _: axum::http::HeaderValue = origin.parse()?;
        }
        Ok(Self {
            http,
            auth,
            address,
            database_url,
            jwt_secret,
            max_connections,
            acquire_timeout: Duration::from_secs(timeout_seconds),
        })
    }
}
