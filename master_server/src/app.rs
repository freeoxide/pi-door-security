use axum::{
    Router,
    routing::get,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::{config::Config, handlers};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health_check))
        .nest("/auth", handlers::auth_router())
        .nest("/users", handlers::users_router())
        .nest("/clients", handlers::clients_router())
        .nest("/clients", handlers::commands_router())
        .nest("/clients", handlers::telemetry_router())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
