//! Configuration management endpoints

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::{ApiContext, ApiError};

#[derive(Serialize)]
struct ConfigResponse {
    system: SystemConfigView,
    network: NetworkConfigView,
    http: HttpConfigView,
    ws_local: WsLocalConfigView,
    cloud: CloudConfigView,
    gpio: GpioConfigView,
    timers: TimerConfigView,
    ble: BleConfigView,
    rf433: Rf433ConfigView,
}

#[derive(Serialize)]
struct SystemConfigView {
    client_id: String,
    data_dir: String,
    log_level: String,
}

#[derive(Serialize)]
struct NetworkConfigView {
    prefer: Vec<String>,
    enable_lte: bool,
}

#[derive(Serialize)]
struct HttpConfigView {
    listen_addr: String,
}

#[derive(Serialize)]
struct WsLocalConfigView {
    enabled: bool,
}

#[derive(Serialize)]
struct CloudConfigView {
    url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    spki_pins: Vec<String>,
    heartbeat_s: u64,
    backoff_min_s: u64,
    backoff_max_s: u64,
    queue_max_events: usize,
    queue_max_age_days: u32,
    #[serde(skip)]
    _secret: String, // JWT token - never serialize
}

#[derive(Serialize)]
struct GpioConfigView {
    reed_in: u8,
    reed_active_low: bool,
    siren_out: u8,
    floodlight_out: u8,
    radio433_rx_in: u8,
    debounce_ms: u64,
}

#[derive(Serialize)]
struct TimerConfigView {
    exit_delay_s: u64,
    entry_delay_s: u64,
    auto_rearm_s: u64,
    siren_max_s: u64,
}

#[derive(Serialize)]
struct BleConfigView {
    enabled: bool,
    pairing_window_s: u64,
}

#[derive(Serialize)]
struct Rf433ConfigView {
    enabled: bool,
    allow_disarm: bool,
    debounce_ms: u64,
}

#[derive(Deserialize)]
pub struct ConfigUpdateRequest {
    #[serde(flatten)]
    config: Value,
}

/// GET /v1/config - Get current configuration with secrets redacted
pub async fn get_config(
    State(_ctx): State<Arc<ApiContext>>,
) -> Result<Json<Value>, ApiError> {
    // Note: In a real implementation, this would load from the actual config
    // For now, return a placeholder response
    Ok(Json(json!({
        "system": {
            "client_id": "pi001",
            "data_dir": "/var/lib/pi-door-client",
            "log_level": "info"
        },
        "network": {
            "prefer": ["eth0", "wlan0"],
            "enable_lte": false
        },
        "http": {
            "listen_addr": "0.0.0.0:8080"
        },
        "ws_local": {
            "enabled": true
        },
        "cloud": {
            "url": "wss://api.example.com/client",
            "spki_pins": [],
            "heartbeat_s": 20,
            "backoff_min_s": 1,
            "backoff_max_s": 60,
            "queue_max_events": 10000,
            "queue_max_age_days": 7
        },
        "gpio": {
            "reed_in": 17,
            "reed_active_low": true,
            "siren_out": 27,
            "floodlight_out": 22,
            "radio433_rx_in": 23,
            "debounce_ms": 50
        },
        "timers": {
            "exit_delay_s": 30,
            "entry_delay_s": 30,
            "auto_rearm_s": 120,
            "siren_max_s": 120
        },
        "ble": {
            "enabled": true,
            "pairing_window_s": 120
        },
        "rf433": {
            "enabled": true,
            "allow_disarm": false,
            "debounce_ms": 500,
            "mappings": []
        }
    })))
}

/// PUT /v1/config - Update configuration (requires restart)
pub async fn update_config(
    State(_ctx): State<Arc<ApiContext>>,
    Json(_request): Json<ConfigUpdateRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // Note: In a real implementation, this would:
    // 1. Validate the configuration
    // 2. Write to disk
    // 3. Mark restart as required
    
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "applied": false,
            "restart_required": true,
            "message": "Configuration update received. Restart required to apply changes."
        }))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use crate::state::new_app_state;

    #[tokio::test]
    async fn test_get_config() {
        let state = new_app_state();
        let (event_bus, _) = EventBus::new();
        let ctx = Arc::new(ApiContext { state, event_bus });

        let result = get_config(State(ctx)).await;
        assert!(result.is_ok());
        
        let json = result.unwrap().0;
        assert!(json["system"]["client_id"].is_string());
        assert!(json["timers"]["exit_delay_s"].is_number());
    }

    #[tokio::test]
    async fn test_update_config() {
        let state = new_app_state();
        let (event_bus, _) = EventBus::new();
        let ctx = Arc::new(ApiContext { state, event_bus });

        let request = ConfigUpdateRequest {
            config: json!({"timers": {"exit_delay_s": 45}}),
        };

        let result = update_config(State(ctx), Json(request)).await;
        assert!(result.is_ok());
        
        let (status, json) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(json["restart_required"], true);
    }
}
