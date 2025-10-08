use axum::{
    Router,
    routing::get,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health_check))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
