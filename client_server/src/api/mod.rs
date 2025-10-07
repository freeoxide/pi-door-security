//! HTTP and WebSocket API module

pub mod handlers;
mod models;
mod error;

pub use models::*;
pub use error::*;

use crate::events::EventBus;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

/// Create the API router
pub fn create_router(state: AppState, event_bus: EventBus) -> Router {
    let ctx = Arc::new(ApiContext { state, event_bus });
    
    Router::new()
        // Health and status
        .route("/v1/health", get(handlers::health))
        .route("/v1/status", get(handlers::get_status))
        // Arm and disarm
        .route("/v1/arm", post(handlers::arm))
        .route("/v1/disarm", post(handlers::disarm))
        // Actuator control
        .route("/v1/siren", post(handlers::control_siren))
        .route("/v1/floodlight", post(handlers::control_floodlight))
        .with_state(ctx)
}

/// Shared API context
pub struct ApiContext {
    pub state: AppState,
    pub event_bus: EventBus,
}
