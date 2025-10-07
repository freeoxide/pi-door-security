//! Arm and disarm endpoint handlers

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::api::{ApiContext, ApiError};
use crate::events::{Event, EventSource};

#[derive(Deserialize)]
pub struct ArmRequest {
    pub exit_delay_s: Option<u64>,
}

#[derive(Serialize)]
pub struct ArmResponse {
    pub state: String,
    pub exit_delay_s: u64,
}

#[derive(Deserialize)]
pub struct DisarmRequest {
    pub auto_rearm_s: Option<u64>,
}

#[derive(Serialize)]
pub struct DisarmResponse {
    pub state: String,
    pub auto_rearm_s: Option<u64>,
}

/// POST /v1/arm - Arm the system
pub async fn arm(
    State(ctx): State<Arc<ApiContext>>,
    Json(req): Json<ArmRequest>,
) -> Result<(StatusCode, Json<ArmResponse>), ApiError> {
    info!(exit_delay_s = ?req.exit_delay_s, "Received arm request");
    
    // Emit arm event
    let event = Event::UserArm {
        source: EventSource::Local,
        exit_delay_s: req.exit_delay_s,
    };
    
    ctx.event_bus.emit(event).map_err(|e| ApiError {
        message: format!("Failed to emit arm event: {}", e),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    
    // Determine exit delay to use
    let exit_delay = req.exit_delay_s.unwrap_or(30);
    
    Ok((
        StatusCode::ACCEPTED,
        Json(ArmResponse {
            state: "exit_delay".to_string(),
            exit_delay_s: exit_delay,
        }),
    ))
}

/// POST /v1/disarm - Disarm the system
pub async fn disarm(
    State(ctx): State<Arc<ApiContext>>,
    Json(req): Json<DisarmRequest>,
) -> Result<(StatusCode, Json<DisarmResponse>), ApiError> {
    info!(auto_rearm_s = ?req.auto_rearm_s, "Received disarm request");
    
    // Emit disarm event
    let event = Event::UserDisarm {
        source: EventSource::Local,
        auto_rearm_s: req.auto_rearm_s,
    };
    
    ctx.event_bus.emit(event).map_err(|e| ApiError {
        message: format!("Failed to emit disarm event: {}", e),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    
    Ok((
        StatusCode::ACCEPTED,
        Json(DisarmResponse {
            state: "disarmed".to_string(),
            auto_rearm_s: req.auto_rearm_s,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use crate::state::new_app_state;

    #[tokio::test]
    async fn test_arm_handler() {
        let state = new_app_state();
        let (event_bus, _rx) = EventBus::new();
        let ctx = Arc::new(ApiContext {
            state,
            event_bus,
        });

        let req = ArmRequest {
            exit_delay_s: Some(30),
        };

        let result = arm(State(ctx), Json(req)).await;
        assert!(result.is_ok());
        
        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(response.state, "exit_delay");
        assert_eq!(response.exit_delay_s, 30);
    }

    #[tokio::test]
    async fn test_disarm_handler() {
        let state = new_app_state();
        let (event_bus, _rx) = EventBus::new();
        let ctx = Arc::new(ApiContext {
            state,
            event_bus,
        });

        let req = DisarmRequest {
            auto_rearm_s: Some(120),
        };

        let result = disarm(State(ctx), Json(req)).await;
        assert!(result.is_ok());
        
        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(response.state, "disarmed");
        assert_eq!(response.auto_rearm_s, Some(120));
    }
}
