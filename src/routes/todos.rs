use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::todo::{CreateTodo, Todo, UpdateTodo},
    state::AppState,
};

pub fn todo_routes() -> Router<AppState> {
    Router::new()
        .route("/todos", get(list_todos).post(create_todo))
        .route(
            "/todos/:id",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
}

async fn list_todos(State(state): State<AppState>) -> Result<Json<Vec<Todo>>, AppError> {
    let todos = sqlx::query_as::<_, Todo>(
        r#"
        SELECT id, title, completed
        FROM todos
        ORDER BY title ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(todos))
}

async fn get_todo(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Todo>, AppError> {
    let todo = sqlx::query_as::<_, Todo>(
        r#"
        SELECT id, title, completed
        FROM todos
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
        .ok_or_else(|| AppError::not_found("Todo not found"))?;

    Ok(Json(todo))
}

async fn create_todo(
    State(state): State<AppState>,
    Json(payload): Json<CreateTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    validate_title(&payload.title)?;

    let todo = Todo {
        id: Uuid::new_v4(),
        title: payload.title.trim().to_owned(),
        completed: false,
    };
    sqlx::query(
        r#"
        INSERT INTO todos (id, title, completed)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(todo.id)
    .bind(&todo.title)
    .bind(todo.completed)
    .execute(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(todo)))
}

async fn update_todo(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateTodo>,
) -> Result<Json<Todo>, AppError> {
    if let Some(title) = payload.title.as_deref() {
        validate_title(title)?;
    }

    let current = sqlx::query_as::<_, Todo>(
        r#"
        SELECT id, title, completed
        FROM todos
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
        .ok_or_else(|| AppError::not_found("Todo not found"))?;

    let title = payload
        .title
        .map(|value| value.trim().to_owned())
        .unwrap_or(current.title);
    let completed = payload.completed.unwrap_or(current.completed);

    let todo = sqlx::query_as::<_, Todo>(
        r#"
        UPDATE todos
        SET title = $2, completed = $3
        WHERE id = $1
        RETURNING id, title, completed
        "#,
    )
    .bind(id)
    .bind(title)
    .bind(completed)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(todo))
}

async fn delete_todo(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let result: PgQueryResult = sqlx::query(
        r#"
        DELETE FROM todos
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    match result.rows_affected() {
        0 => Err(AppError::not_found("Todo not found")),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::bad_request("Title must not be empty"));
    }

    Ok(())
}
