use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use {{crate_name}}::{app, state::AppState};

fn router(db: PgPool) -> Router {
    app(AppState {
        db,
        http: Default::default(),
        rate_limiter: Default::default(),
        auth: Default::default(),
        jwt_secret: "test-secret-at-least-thirty-two-bytes-long".into(),
        password_tasks: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
    })
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&body).unwrap_or(Value::Null),
    )
}

async fn register(app: &Router, email: &str) -> (String, String) {
    let (status, response) = call(
        app,
        "POST",
        "/api/auth/register",
        json!({
            "email": email,
            "password": "a-long-development-password"
        }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "registration failed: {response}");
    (
        response["access_token"].as_str().unwrap().to_owned(),
        response["user"]["id"].as_str().unwrap().to_owned(),
    )
}

#[tokio::test]
async fn public_routes_and_auth_are_bounded() {
    let db = PgPoolOptions::new()
        .connect_lazy("postgres://none:none@127.0.0.1:1/none")
        .unwrap();
    let app = router(db);
    assert_eq!(
        call(&app, "GET", "/health", Value::Null, None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "GET", "/api/projects", Value::Null, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, "GET", "/docs", Value::Null, None).await.0,
        StatusCode::OK
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn users_own_projects_and_tasks(pool: PgPool) {
    let app = router(pool);
    let (owner_token, _) = register(&app, "owner@example.com").await;
    let (other_token, _) = register(&app, "other@example.com").await;

    let (status, project) = call(
        &app,
        "POST",
        "/api/projects",
        json!({"name":"Launch","description":"First release"}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "project creation failed: {project}");
    let project_id = project["id"].as_str().unwrap();
    let project_path = format!("/api/projects/{project_id}");
    assert_eq!(
        call(&app, "GET", &project_path, Value::Null, Some(&other_token))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (status, updated_project) = call(
        &app,
        "PATCH",
        &project_path,
        json!({"name":"Launch v2"}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_project["name"], "Launch v2");
    assert_eq!(updated_project["description"], "First release");
    let (status, cleared_project) = call(
        &app,
        "PATCH",
        &project_path,
        json!({"description":null}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(cleared_project["description"].is_null());
    assert_eq!(
        call(
            &app,
            "PATCH",
            &project_path,
            json!({"name":null}),
            Some(&owner_token)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let invalid_date = call(
        &app,
        "POST",
        &format!("{project_path}/tasks"),
        json!({"title":"Bad date","due_date":"2027-02-29"}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(invalid_date.0, StatusCode::BAD_REQUEST);

    let long_description = "x".repeat(501);
    let invalid_description = call(
        &app,
        "POST",
        &format!("{project_path}/tasks"),
        json!({"title":"Long description","description":long_description}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(invalid_description.0, StatusCode::BAD_REQUEST);

    let invalid_priority = call(
        &app,
        "POST",
        &format!("{project_path}/tasks"),
        json!({"title":"Bad priority","priority":0}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(invalid_priority.0, StatusCode::BAD_REQUEST);

    let (status, task) = call(
        &app,
        "POST",
        &format!("{project_path}/tasks"),
        json!({
            "title":"Ship",
            "priority":1,
            "due_date":"2028-02-29"
        }),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "task creation failed: {task}");
    assert_eq!(task["due_date"], "2028-02-29");

    let task_path = format!(
        "{project_path}/tasks/{}",
        task["id"].as_str().unwrap()
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &task_path,
            json!({"status":"DONE"}),
            Some(&other_token)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &task_path,
            json!({"status":"NOT_A_STATUS"}),
            Some(&owner_token)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &task_path,
            json!({"due_date":"2028-02-30"}),
            Some(&owner_token)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, updated) = call(
        &app,
        "PATCH",
        &task_path,
        json!({"status":"DONE"}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["status"], "DONE");
    assert_eq!(updated["due_date"], "2028-02-29");
    let (status, cleared_task) = call(
        &app,
        "PATCH",
        &task_path,
        json!({"description":null,"due_date":null}),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(cleared_task["description"].is_null());
    assert!(cleared_task["due_date"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_rotation_and_logout_revoke_sessions(pool: PgPool) {
    let app = router(pool);
    let (_, user_id) = register(&app, "sessions@example.com").await;
    let (login_status, initial) = call(
        &app,
        "POST",
        "/api/auth/login",
        json!({
            "email":"sessions@example.com",
            "password":"a-long-development-password"
        }),
        None,
    )
    .await;
    assert_eq!(login_status, StatusCode::OK, "login failed: {initial}");
    let original_access = initial["access_token"].as_str().unwrap();
    let original_refresh = initial["refresh_token"].as_str().unwrap();

    let (status, rotated) = call(
        &app,
        "POST",
        "/api/auth/refresh",
        json!({"refresh_token":original_refresh}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "refresh failed: {rotated}");
    assert_ne!(rotated["refresh_token"], original_refresh);
    assert_eq!(rotated["user"]["id"], user_id);
    let new_access = rotated["access_token"].as_str().unwrap();
    let new_refresh = rotated["refresh_token"].as_str().unwrap();

    assert_eq!(
        call(
            &app,
            "POST",
            "/api/auth/refresh",
            json!({"refresh_token":original_refresh}),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            "GET",
            "/api/auth/me",
            Value::Null,
            Some(original_access)
        )
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, "GET", "/api/auth/me", Value::Null, Some(new_access))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/auth/logout",
            Value::Null,
            Some(new_access)
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, "GET", "/api/auth/me", Value::Null, Some(new_access))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/auth/refresh",
            json!({"refresh_token":new_refresh}),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn user_management_requires_admin_and_admin_can_manage_roles(pool: PgPool) {
    let app = router(pool.clone());
    let (admin_token, id) = register(&app, "admin@example.com").await;
    let admin_id: uuid::Uuid = id.parse().unwrap();

    sqlx::query("UPDATE users SET role='ADMIN' WHERE id=$1")
        .bind(admin_id)
        .execute(&pool)
        .await
        .unwrap();

    let (user_token, user_id) = register(&app, "member@example.com").await;
    assert_eq!(
        call(&app, "GET", "/api/users", Value::Null, Some(&user_token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!("/api/users/{user_id}/role"),
            json!({"role":"ADMIN"}),
            Some(&user_token)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, created) = call(
        &app,
        "POST",
        "/api/users",
        json!({
            "email":"managed@example.com",
            "password":"a-long-development-password",
            "role":"USER"
        }),
        Some(&admin_token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "admin create failed: {created}");
    let managed_id = created["id"].as_str().unwrap();

    let (status, disabled) = call(
        &app,
        "PATCH",
        &format!("/api/users/{managed_id}/role"),
        json!({"role":"USER","disabled":true}),
        Some(&admin_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(disabled["disabled"], true);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/auth/login",
            json!({
                "email":"managed@example.com",
                "password":"a-long-development-password"
            }),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );

    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!("/api/users/{admin_id}/role"),
            json!({"role":"USER"}),
            Some(&admin_token)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}
