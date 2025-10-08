mod app;
mod config;
mod db;
mod entities;

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::app::{create_router, AppState};
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "master_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Load configuration
    let config = Config::from_env();
    tracing::info!("Configuration loaded");

    // Connect to database and run migrations
    let db = db::connect(&config.database_url).await?;

    // Create application state
    let state = AppState {
        db,
        config: Arc::new(config.clone()),
    };

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&config.server_bind).await?;
    tracing::info!("Server listening on {}", config.server_bind);

    axum::serve(listener, app).await?;

    Ok(())
}
