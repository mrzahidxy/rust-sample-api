use crate::{
    error::AppError,
    models::user::User,
    pagination::Pagination,
    services::users::{self, CreateUser, UpdateRole, UpdateUser},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch},
    Extension, Json, Router,
};
use uuid::Uuid;
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list).post(create))
        .route("/users/:id", get(get_one).patch(update).delete(remove))
        .route("/users/:id/role", patch(role))
}
async fn list(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<User>>, AppError> {
    Ok(Json(users::list(&s, &u, p.limit, p.offset()?).await?))
}
async fn create(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(p): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), AppError> {
    Ok((StatusCode::CREATED, Json(users::create(&s, &u, p).await?)))
}
async fn get_one(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    Ok(Json(users::get(&s, &u, id).await?))
}
async fn update(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
    Json(p): Json<UpdateUser>,
) -> Result<Json<User>, AppError> {
    Ok(Json(users::update(&s, &u, id, p).await?))
}
async fn role(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
    Json(p): Json<UpdateRole>,
) -> Result<Json<User>, AppError> {
    Ok(Json(users::role(&s, &u, id, p).await?))
}
async fn remove(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    users::remove(&s, &u, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
