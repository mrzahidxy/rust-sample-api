use {{crate_name}}::{
    config::Config,
    state::{AppState, MIGRATOR},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let state = AppState::connect(&Config::from_env()?).await?;
    MIGRATOR.run(&state.db).await?;
    state.db.close().await;
    println!("Migrations complete.");
    Ok(())
}
