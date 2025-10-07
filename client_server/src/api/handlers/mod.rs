//! API request handlers

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
