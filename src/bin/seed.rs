#[path = "../state.rs"]
mod state;

use std::env;

use dotenvy::dotenv;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set before running the seed");

    println!("Connecting to the database...");

    let app_state = state::AppState::new(&database_url)
        .await
        .expect("failed to connect to PostgreSQL");

    println!("Clearing existing todos...");

    sqlx::query("DELETE FROM todos")
        .execute(&app_state.db)
        .await
        .expect("failed to clear todos table");

    let seed_todos = [
        (
            Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("valid UUID"),
            "Learn Rust basics",
            false,
        ),
        (
            Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("valid UUID"),
            "Build an Axum API",
            true,
        ),
        (
            Uuid::parse_str("33333333-3333-3333-3333-333333333333").expect("valid UUID"),
            "Practice PostgreSQL queries",
            false,
        ),
    ];

    println!("Inserting sample todos...");

    for (id, title, completed) in seed_todos {
        sqlx::query(
            r#"
            INSERT INTO todos (id, title, completed)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(completed)
        .execute(&app_state.db)
        .await
        .expect("failed to insert seed todo");

        println!("- Added: {title}");
    }

    println!();
    println!("Database seeding complete.");
    println!("You can now run:");
    println!("  cargo run");
    println!("Then test:");
    println!("  curl http://127.0.0.1:3000/todos");
}
