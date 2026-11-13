// Compatibility exports for consumers of the original starter.
pub use crate::middleware::auth::require_auth;
pub use crate::services::auth::{Claims, Credentials, TokenResponse};
