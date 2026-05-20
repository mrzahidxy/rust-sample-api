mod error;
mod models;
mod routes;
mod state;

use std::{env, net::SocketAddr};

use axum::Router;
use dotenvy::dotenv;
use tokio::net::TcpListener;

use crate::{
    routes::{health::health_routes, todos::todo_routes},
    state::AppState,
};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set before starting the server");

    let app_state = AppState::new(&database_url)
        .await
        .expect("failed to connect to PostgreSQL");

    let app = Router::new()
        .merge(health_routes())
        .merge(todo_routes())
        .with_state(app_state);
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));

    println!("Server running on http://{address}");

    let listener = TcpListener::bind(address)
        .await
        .expect("failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("failed to start HTTP server");
}
