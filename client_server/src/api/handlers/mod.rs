//! API request handlers

mod status;
mod arm_disarm;
mod actuators;
mod websocket;
mod config;
mod ble;

pub use status::get_status;
pub use arm_disarm::{arm, disarm};
pub use actuators::{control_siren, control_floodlight};
pub use websocket::websocket_handler;
pub use config::{get_config, update_config};
pub use ble::ble_pairing;

use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::ApiContext;

/// Health check endpoint
pub async fn health(
    State(ctx): State<Arc<ApiContext>>,
) -> Json<Value> {
    let state = ctx.state.read();
    
    Json(json!({
        "status": "ok",
        "ready": true,
        "uptime_s": state.uptime_s(),
        "version": crate::VERSION,
    }))
}
