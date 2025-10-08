//! BLE pairing management endpoints

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

use crate::api::{ApiContext, ApiError};

#[derive(Deserialize)]
pub struct BlePairingRequest {
    pub enable: bool,
    #[serde(default = "default_pairing_seconds")]
    pub seconds: u64,
}

fn default_pairing_seconds() -> u64 {
    120
}

#[derive(Serialize)]
pub struct BlePairingResponse {
    pub enabled: bool,
    pub expires_in_s: Option<u64>,
    pub message: String,
}

/// POST /v1/ble/pairing - Enable/disable BLE pairing mode
pub async fn ble_pairing(
    State(_ctx): State<Arc<ApiContext>>,
    Json(request): Json<BlePairingRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    info!(
        enable = request.enable,
        duration_s = request.seconds,
        "BLE pairing mode request"
    );

    if request.enable {
        // In a real implementation, this would:
        // 1. Enable BLE discoverable/pairable mode
        // 2. Set a timer to disable after the specified duration
        // 3. Return the pairing PIN if numeric passkey is used
        
        Ok((
            StatusCode::ACCEPTED,
            Json(json!({
                "enabled": true,
                "expires_in_s": request.seconds,
                "message": format!("BLE pairing mode enabled for {} seconds", request.seconds),
                "note": "BLE pairing requires hardware support and is not active in mock mode"
            }))
        ))
    } else {
        // Disable pairing mode
        Ok((
            StatusCode::ACCEPTED,
            Json(json!({
                "enabled": false,
                "expires_in_s": null,
                "message": "BLE pairing mode disabled"
            }))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::events::EventBus;
    use crate::state::new_app_state;

    #[tokio::test]
    async fn test_enable_ble_pairing() {
        let state = new_app_state();
        let (event_bus, _) = EventBus::new();
        let config = AppConfig::test_default();
        let ctx = Arc::new(ApiContext { state, event_bus, config });

        let request = BlePairingRequest {
            enable: true,
            seconds: 120,
        };

        let result = ble_pairing(State(ctx), Json(request)).await;
        assert!(result.is_ok());
        
        let (status, json) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(json["enabled"], true);
        assert_eq!(json["expires_in_s"], 120);
    }

    #[tokio::test]
    async fn test_disable_ble_pairing() {
        let state = new_app_state();
        let (event_bus, _) = EventBus::new();
        let config = AppConfig::test_default();
        let ctx = Arc::new(ApiContext { state, event_bus, config });

        let request = BlePairingRequest {
            enable: false,
            seconds: 0,
        };

        let result = ble_pairing(State(ctx), Json(request)).await;
        assert!(result.is_ok());
        
        let (status, json) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(json["enabled"], false);
    }
}
