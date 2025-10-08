//! Actuator control endpoint handlers

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::api::{ApiContext, ApiError};
use crate::events::Event;

#[derive(Deserialize)]
pub struct SirenRequest {
    pub on: bool,
    pub duration_s: Option<u64>,
}

#[derive(Serialize)]
pub struct SirenResponse {
    pub actuators: ActuatorsStatus,
    pub duration_s: Option<u64>,
}

#[derive(Deserialize)]
pub struct FloodlightRequest {
    pub on: bool,
    pub duration_s: Option<u64>,
}

#[derive(Serialize)]
pub struct FloodlightResponse {
    pub actuators: ActuatorsStatus,
    pub duration_s: Option<u64>,
}

#[derive(Serialize)]
pub struct ActuatorsStatus {
    pub siren: bool,
    pub floodlight: bool,
}

/// POST /v1/siren - Control siren
pub async fn control_siren(
    State(ctx): State<Arc<ApiContext>>,
    Json(req): Json<SirenRequest>,
) -> Result<(StatusCode, Json<SirenResponse>), ApiError> {
    info!(on = req.on, duration_s = ?req.duration_s, "Received siren control request");
    
    // Emit siren control event
    let event = Event::SirenControl {
        on: req.on,
        duration_s: req.duration_s,
    };
    
    ctx.event_bus.emit(event).map_err(|e| ApiError {
        message: format!("Failed to emit siren control event: {}", e),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    
    // Get current actuator state
    let state = ctx.state.read();
    
    Ok((
        StatusCode::ACCEPTED,
        Json(SirenResponse {
            actuators: ActuatorsStatus {
                siren: state.actuators.siren,
                floodlight: state.actuators.floodlight,
            },
            duration_s: req.duration_s,
        }),
    ))
}

/// POST /v1/floodlight - Control floodlight
pub async fn control_floodlight(
    State(ctx): State<Arc<ApiContext>>,
    Json(req): Json<FloodlightRequest>,
) -> Result<(StatusCode, Json<FloodlightResponse>), ApiError> {
    info!(on = req.on, duration_s = ?req.duration_s, "Received floodlight control request");
    
    // Emit floodlight control event
    let event = Event::FloodlightControl {
        on: req.on,
        duration_s: req.duration_s,
    };
    
    ctx.event_bus.emit(event).map_err(|e| ApiError {
        message: format!("Failed to emit floodlight control event: {}", e),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    
    // Get current actuator state
    let state = ctx.state.read();
    
    Ok((
        StatusCode::ACCEPTED,
        Json(FloodlightResponse {
            actuators: ActuatorsStatus {
                siren: state.actuators.siren,
                floodlight: state.actuators.floodlight,
            },
            duration_s: req.duration_s,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::events::EventBus;
    use crate::state::new_app_state;

    #[tokio::test]
    async fn test_siren_control() {
        let state = new_app_state();
        let (event_bus, _rx) = EventBus::new();
        let config = AppConfig::test_default();
        let ctx = Arc::new(ApiContext {
            state,
            event_bus,
            config,
        });

        let req = SirenRequest {
            on: true,
            duration_s: Some(60),
        };

        let result = control_siren(State(ctx), Json(req)).await;
        assert!(result.is_ok());
        
        let (status, _response) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_floodlight_control() {
        let state = new_app_state();
        let (event_bus, _rx) = EventBus::new();
        let config = AppConfig::test_default();
        let ctx = Arc::new(ApiContext {
            state,
            event_bus,
            config,
        });

        let req = FloodlightRequest {
            on: true,
            duration_s: Some(600),
        };

        let result = control_floodlight(State(ctx), Json(req)).await;
        assert!(result.is_ok());
        
        let (status, _response) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
    }
}
