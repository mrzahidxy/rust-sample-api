use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
    pub disabled: bool,
}
impl User {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.role == "ADMIN" {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}
