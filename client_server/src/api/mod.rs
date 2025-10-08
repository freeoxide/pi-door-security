//! HTTP and WebSocket API module

pub mod handlers;
mod models;
mod error;

pub use models::*;
pub use error::*;

use crate::config::AppConfig;
use crate::events::EventBus;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post, put},
};
use std::sync::Arc;

/// Create the API router
pub fn create_router(state: AppState, event_bus: EventBus, config: AppConfig) -> Router {
    let ctx = Arc::new(ApiContext { state, event_bus, config });
    
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
        // Configuration management
        .route("/v1/config", get(handlers::get_config))
        .route("/v1/config", put(handlers::update_config))
        // BLE pairing
        .route("/v1/ble/pairing", post(handlers::ble_pairing))
        // WebSocket for real-time events
        .route("/v1/ws", get(handlers::websocket_handler))
        .with_state(ctx)
}

/// Shared API context
pub struct ApiContext {
    pub state: AppState,
    pub event_bus: EventBus,
    pub config: AppConfig,
}
