use crate::{
    credentials::normalize_email,
    error::AppError,
    models::user::User,
    services::crate_role::Role,
    services::auth,
    state::AppState,
};
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub role: Role,
}
#[derive(Deserialize)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub name: Option<String>,
}
#[derive(Deserialize)]
pub struct UpdateRole {
    pub role: Role,
    pub disabled: Option<bool>,
}

pub async fn get(state: &AppState, actor: &User, id: Uuid) -> Result<User, AppError> {
    if actor.id != id {
        actor.require_admin()?;
    }
    sqlx::query_as("SELECT id,email,name,role,disabled FROM users WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("User not found"))
}
pub async fn create(state: &AppState, actor: &User, input: CreateUser) -> Result<User, AppError> {
    actor.require_admin()?;
    let email = normalize_email(&input.email).map_err(AppError::bad_request)?;
    let name = auth::validate_name(input.name)?;
    let hash = auth::hash_password(state, input.password).await?;
    Ok(sqlx::query_as("INSERT INTO users(id,email,password_hash,name,role) VALUES($1,$2,$3,$4,$5) RETURNING id,email,name,role,disabled")
        .bind(Uuid::new_v4()).bind(email).bind(hash).bind(name).bind(input.role.as_str()).fetch_one(&state.db).await?)
}
pub async fn update(
    state: &AppState,
    actor: &User,
    id: Uuid,
    input: UpdateUser,
) -> Result<User, AppError> {
    if actor.id != id {
        actor.require_admin()?;
    }
    let email = input
        .email
        .as_deref()
        .map(normalize_email)
        .transpose()
        .map_err(AppError::bad_request)?;
    let name = auth::validate_name(input.name)?;
    sqlx::query_as("UPDATE users SET email=COALESCE($2,email),name=COALESCE($3,name) WHERE id=$1 RETURNING id,email,name,role,disabled")
        .bind(id).bind(email).bind(name).fetch_optional(&state.db).await?.ok_or_else(||AppError::not_found("User not found"))
}
pub async fn role(
    state: &AppState,
    actor: &User,
    id: Uuid,
    input: UpdateRole,
) -> Result<User, AppError> {
    actor.require_admin()?;
    // Keep the current administrator able to recover the installation.
    if actor.id == id && (!matches!(input.role, Role::Admin) || input.disabled == Some(true)) {
        return Err(AppError::bad_request(
            "Cannot demote or disable your own administrator account",
        ));
    }
    sqlx::query_as("UPDATE users SET role=$2,disabled=COALESCE($3,disabled) WHERE id=$1 RETURNING id,email,name,role,disabled")
        .bind(id).bind(input.role.as_str()).bind(input.disabled).fetch_optional(&state.db).await?.ok_or_else(||AppError::not_found("User not found"))
}
pub async fn remove(state: &AppState, actor: &User, id: Uuid) -> Result<(), AppError> {
    actor.require_admin()?;
    if actor.id == id {
        return Err(AppError::bad_request(
            "Cannot delete your own administrator account",
        ));
    }
    if sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected()
        == 0
    {
        return Err(AppError::not_found("User not found"));
    }
    Ok(())
}

pub async fn list(
    state: &AppState,
    actor: &User,
    limit: i64,
    offset: i64,
) -> Result<Vec<User>, AppError> {
    actor.require_admin()?;
    Ok(sqlx::query_as(
        "SELECT id,email,name,role,disabled FROM users ORDER BY id LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?)
}
