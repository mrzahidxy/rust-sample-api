use crate::{
    error::AppError,
    models::{
        project::{Project, ProjectInput, ProjectUpdate, Task, TaskInput, TaskUpdate},
        user::User,
    },
    pagination::Pagination,
    services::projects,
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
        .route("/projects", get(list).post(create))
        .route("/projects/:id", get(read).patch(update).delete(remove))
        .route("/projects/:id/tasks", get(tasks).post(create_task))
        .route(
            "/projects/:id/tasks/:task",
            patch(update_task).delete(delete_task),
        )
}
async fn list(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<Project>>, AppError> {
    Ok(Json(projects::projects(&s.db, u.id, p).await?))
}
async fn create(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(p): Json<ProjectInput>,
) -> Result<(StatusCode, Json<Project>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(projects::create_project(&s.db, u.id, p).await?),
    ))
}
async fn read(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<Project>, AppError> {
    Ok(Json(projects::project(&s.db, u.id, id).await?))
}
async fn update(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
    Json(p): Json<ProjectUpdate>,
) -> Result<Json<Project>, AppError> {
    Ok(Json(projects::update_project(&s.db, u.id, id, p).await?))
}
async fn remove(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    projects::delete_project(&s.db, u.id, id).await
}
async fn tasks(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<Task>>, AppError> {
    Ok(Json(projects::tasks(&s.db, u.id, id, p).await?))
}
async fn create_task(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<Uuid>,
    Json(p): Json<TaskInput>,
) -> Result<(StatusCode, Json<Task>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(projects::create_task(&s.db, u.id, id, p).await?),
    ))
}
async fn update_task(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path((id, task)): Path<(Uuid, Uuid)>,
    Json(p): Json<TaskUpdate>,
) -> Result<Json<Task>, AppError> {
    Ok(Json(projects::update_task(&s.db, u.id, id, task, p).await?))
}
async fn delete_task(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path((id, task)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    projects::delete_task(&s.db, u.id, id, task).await
}
