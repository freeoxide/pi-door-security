//! HTTP and WebSocket API module

pub mod handlers;
mod models;
mod error;

pub use models::*;
pub use error::*;

use crate::events::EventBus;
use crate::state::AppState;
use axum::{Router, routing::get};
use std::sync::Arc;

/// Create the API router
pub fn create_router(state: AppState, event_bus: EventBus) -> Router {
    Router::new()
        .route("/v1/health", get(handlers::health))
        .with_state(Arc::new(ApiContext { state, event_bus }))
}

/// Shared API context
pub struct ApiContext {
    pub state: AppState,
    pub event_bus: EventBus,
}
