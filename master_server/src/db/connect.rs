use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use migration::{Migrator, MigratorTrait};
use anyhow::Result;
use tracing::log;

/// Establishes a connection to the database using SeaORM and runs migrations
pub async fn connect(database_url: &str) -> Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(3600))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Info);

    let conn = Database::connect(opt).await?;

    conn.ping().await?;
    tracing::info!("Database connection established");

    Migrator::up(&conn, None).await?;
    tracing::info!("Database migrations completed");

    Ok(conn)
}
