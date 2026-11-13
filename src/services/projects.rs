use crate::{
    error::AppError,
    models::project::{Patch, Project, ProjectInput, ProjectUpdate, Task, TaskInput, TaskUpdate},
    pagination::Pagination,
};
use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

const MAX_TEXT_BYTES: usize = 500;

fn required_text(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_TEXT_BYTES {
        return Err(AppError::bad_request(format!(
            "{field} must contain 1–{MAX_TEXT_BYTES} bytes"
        )));
    }
    Ok(value.to_owned())
}

fn optional_text(value: Option<String>, field: &str) -> Result<Option<String>, AppError> {
    value
        .map(|value| {
            let value = value.trim();
            if value.len() > MAX_TEXT_BYTES {
                return Err(AppError::bad_request(format!(
                    "{field} must be at most {MAX_TEXT_BYTES} bytes"
                )));
            }
            Ok((!value.is_empty()).then(|| value.to_owned()))
        })
        .transpose()
        .map(Option::flatten)
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(i, byte)| matches!(i, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }

    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

fn optional_date(value: Option<String>) -> Result<Option<String>, AppError> {
    value
        .map(|date| {
            if !valid_date(&date) {
                return Err(AppError::bad_request(
                    "due_date must be a valid date in YYYY-MM-DD format",
                ));
            }
            Ok(date)
        })
        .transpose()
}

fn required_patch(value: Patch<String>, field: &str) -> Result<Option<String>, AppError> {
    match value {
        Patch::Missing => Ok(None),
        Patch::Null => Err(AppError::bad_request(format!("{field} cannot be null"))),
        Patch::Value(value) => required_text(&value, field).map(Some),
    }
}

fn optional_text_patch(
    value: Patch<String>,
    field: &str,
) -> Result<Option<Option<String>>, AppError> {
    match value {
        Patch::Missing => Ok(None),
        Patch::Null => Ok(Some(None)),
        Patch::Value(value) => optional_text(Some(value), field).map(Some),
    }
}

fn optional_date_patch(value: Patch<String>) -> Result<Option<Option<String>>, AppError> {
    match value {
        Patch::Missing => Ok(None),
        Patch::Null => Ok(Some(None)),
        Patch::Value(value) => optional_date(Some(value)).map(Some),
    }
}

pub async fn projects(
    db: &PgPool,
    user: Uuid,
    page: Pagination,
) -> Result<Vec<Project>, AppError> {
    Ok(sqlx::query_as(
        "SELECT id,name,description,owner_id FROM projects WHERE owner_id=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(user)
    .bind(page.limit)
    .bind(page.offset()?)
    .fetch_all(db)
    .await?)
}

pub async fn create_project(
    db: &PgPool,
    user: Uuid,
    project: ProjectInput,
) -> Result<Project, AppError> {
    Ok(sqlx::query_as(
        "INSERT INTO projects(id,name,description,owner_id) VALUES($1,$2,$3,$4) RETURNING id,name,description,owner_id",
    )
    .bind(Uuid::new_v4())
    .bind(required_text(&project.name, "name")?)
    .bind(optional_text(project.description, "description")?)
    .bind(user)
    .fetch_one(db)
    .await?)
}

pub async fn project(db: &PgPool, user: Uuid, id: Uuid) -> Result<Project, AppError> {
    sqlx::query_as(
        "SELECT id,name,description,owner_id FROM projects WHERE id=$1 AND owner_id=$2",
    )
    .bind(id)
    .bind(user)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("Project not found"))
}

pub async fn update_project(
    db: &PgPool,
    user: Uuid,
    id: Uuid,
    project: ProjectUpdate,
) -> Result<Project, AppError> {
    let name = required_patch(project.name, "name")?;
    let description = optional_text_patch(project.description, "description")?;
    sqlx::query_as(
        "UPDATE projects SET name=CASE WHEN $3 THEN $4 ELSE name END,description=CASE WHEN $5 THEN $6 ELSE description END,updated_at=now() WHERE id=$1 AND owner_id=$2 RETURNING id,name,description,owner_id",
    )
    .bind(id)
    .bind(user)
    .bind(name.is_some())
    .bind(name)
    .bind(description.is_some())
    .bind(description.flatten())
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("Project not found"))
}

pub async fn delete_project(db: &PgPool, user: Uuid, id: Uuid) -> Result<StatusCode, AppError> {
    if sqlx::query("DELETE FROM projects WHERE id=$1 AND owner_id=$2")
        .bind(id)
        .bind(user)
        .execute(db)
        .await?
        .rows_affected()
        == 0
    {
        return Err(AppError::not_found("Project not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn tasks(
    db: &PgPool,
    user: Uuid,
    id: Uuid,
    page: Pagination,
) -> Result<Vec<Task>, AppError> {
    project(db, user, id).await?;
    Ok(sqlx::query_as(
        "SELECT t.id,t.project_id,t.title,t.description,t.status,t.priority,t.due_date::text AS due_date FROM tasks t JOIN projects p ON p.id=t.project_id WHERE p.id=$1 AND p.owner_id=$2 ORDER BY t.created_at DESC LIMIT $3 OFFSET $4",
    )
    .bind(id)
    .bind(user)
    .bind(page.limit)
    .bind(page.offset()?)
    .fetch_all(db)
    .await?)
}

pub async fn create_task(
    db: &PgPool,
    user: Uuid,
    id: Uuid,
    task: TaskInput,
) -> Result<Task, AppError> {
    project(db, user, id).await?;
    let priority = task.priority.unwrap_or(3);
    if !(1..=5).contains(&priority) {
        return Err(AppError::bad_request("priority must be 1–5"));
    }
    Ok(sqlx::query_as(
        "INSERT INTO tasks(id,project_id,title,description,priority,due_date) VALUES($1,$2,$3,$4,$5,$6::date) RETURNING id,project_id,title,description,status,priority,due_date::text AS due_date",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .bind(required_text(&task.title, "title")?)
    .bind(optional_text(task.description, "description")?)
    .bind(priority)
    .bind(optional_date(task.due_date)?)
    .fetch_one(db)
    .await?)
}

pub async fn update_task(
    db: &PgPool,
    user: Uuid,
    id: Uuid,
    task: Uuid,
    update: TaskUpdate,
) -> Result<Task, AppError> {
    if let Some(status) = update.status.as_deref() {
        if !["TODO", "IN_PROGRESS", "BLOCKED", "DONE"].contains(&status) {
            return Err(AppError::bad_request("Invalid task status"));
        }
    }
    if update.priority.is_some_and(|value| !(1..=5).contains(&value)) {
        return Err(AppError::bad_request("priority must be 1–5"));
    }

    let title = update
        .title
        .map(|value| required_text(&value, "title"))
        .transpose()?;
    let description = optional_text_patch(update.description, "description")?;
    let due_date = optional_date_patch(update.due_date)?;

    sqlx::query_as(
        "UPDATE tasks t SET title=COALESCE($4,t.title),description=CASE WHEN $5 THEN $6 ELSE t.description END,status=COALESCE($7,t.status),priority=COALESCE($8,t.priority),due_date=CASE WHEN $9 THEN $10::date ELSE t.due_date END,updated_at=now() FROM projects p WHERE t.id=$1 AND t.project_id=$2 AND p.id=t.project_id AND p.owner_id=$3 RETURNING t.id,t.project_id,t.title,t.description,t.status,t.priority,t.due_date::text AS due_date",
    )
    .bind(task)
    .bind(id)
    .bind(user)
    .bind(title)
    .bind(description.is_some())
    .bind(description.flatten())
    .bind(update.status)
    .bind(update.priority)
    .bind(due_date.is_some())
    .bind(due_date.flatten())
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("Task not found"))
}

pub async fn delete_task(
    db: &PgPool,
    user: Uuid,
    id: Uuid,
    task: Uuid,
) -> Result<StatusCode, AppError> {
    if sqlx::query("DELETE FROM tasks t USING projects p WHERE t.id=$1 AND t.project_id=$2 AND p.id=t.project_id AND p.owner_id=$3")
        .bind(task)
        .bind(id)
        .bind(user)
        .execute(db)
        .await?
        .rows_affected()
        == 0
    {
        return Err(AppError::not_found("Task not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}
