use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use uuid::Uuid;
use {{crate_name}}::{config::Config, credentials::{normalize_email, validate_password}, state::{AppState, MIGRATOR}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let email = normalize_email(&std::env::var("SEED_EMAIL")?)?;
    let password = std::env::var("SEED_PASSWORD")?;
    validate_password(&password)?;
    let role = std::env::var("SEED_ROLE").unwrap_or_else(|_| "USER".into());
    if !["ADMIN", "USER"].contains(&role.as_str()) {
        return Err("SEED_ROLE must be ADMIN or USER".into());
    }
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|_| std::io::Error::other("Password hashing failed"))?
        .to_string();
    let state = AppState::connect(&Config::from_env()?).await?;
    MIGRATOR.run(&state.db).await?;
    sqlx::query("INSERT INTO users (id,email,password_hash,role) VALUES ($1,$2,$3,$4) ON CONFLICT (email) DO NOTHING")
        .bind(Uuid::new_v4()).bind(email).bind(password_hash).bind(role).execute(&state.db).await?;
    state.db.close().await;
    println!("Seed user created if the email was new; existing users are unchanged.");
    Ok(())
}
