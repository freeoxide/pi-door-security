use anyhow::Result;
use sea_orm::{ActiveModelTrait, Database, Set};
use std::io::{self, Write};
use uuid::Uuid;

// Share modules with main binary
#[path = "../auth/password.rs"]
mod password;

#[path = "../config.rs"]
mod config;

#[path = "../entities/mod.rs"]
mod entities;

use entities::users;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Master Server CLI - masterctl");
    println!();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: masterctl <command>");
        println!("Commands:");
        println!("  bootstrap-admin  - Create the first admin user");
        return Ok(());
    }

    match args[1].as_str() {
        "bootstrap-admin" => bootstrap_admin().await?,
        _ => {
            println!("Unknown command: {}", args[1]);
            println!("Run 'masterctl' without arguments for usage.");
        }
    }

    Ok(())
}

async fn bootstrap_admin() -> Result<()> {
    println!("=== Bootstrap Admin User ===");
    println!();

    // Load config
    let config = config::Config::from_env();

    // Connect to database
    println!("Connecting to database...");
    let db = Database::connect(&config.database_url).await?;
    println!("Connected!");
    println!();

    // Get username
    print!("Enter admin username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    if username.is_empty() {
        anyhow::bail!("Username cannot be empty");
    }

    // Get password
    print!("Enter admin password: ");
    io::stdout().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();

    if password.len() < 8 {
        anyhow::bail!("Password must be at least 8 characters");
    }

    // Hash password
    println!("Hashing password...");
    let password_hash = password::hash_password(&password)?;

    // Create user
    println!("Creating admin user...");
    let user = users::ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(username.clone()),
        password_hash: Set(password_hash),
        role: Set(users::UserRole::Admin),
        otp_secret: Set(None),
        otp_enabled: Set(false),
        created_at: Set(chrono::Utc::now().into()),
    };

    user.insert(&db).await?;

    println!();
    println!("✓ Admin user '{}' created successfully!", username);
    println!();
    println!("You can now login with:");
    println!("  Username: {}", username);
    println!("  Password: <the password you entered>");

    Ok(())
}
