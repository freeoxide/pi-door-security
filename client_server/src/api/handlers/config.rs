//! Configuration management endpoints

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::{ApiContext, ApiError};

#[derive(Serialize)]
pub struct ConfigResponse {
    pub system: SystemConfigView,
    pub network: NetworkConfigView,
    pub http: HttpConfigView,
    pub ws_local: WsLocalConfigView,
    pub cloud: CloudConfigView,
    pub gpio: GpioConfigView,
    pub timers: TimerConfigView,
    pub ble: BleConfigView,
    pub rf433: Rf433ConfigView,
}

#[derive(Serialize)]
pub struct SystemConfigView {
    pub client_id: String,
    pub data_dir: String,
    pub log_level: String,
}

#[derive(Serialize)]
pub struct NetworkConfigView {
    pub prefer: Vec<String>,
    pub enable_lte: bool,
}

#[derive(Serialize)]
pub struct HttpConfigView {
    pub listen_addr: String,
}

#[derive(Serialize)]
pub struct WsLocalConfigView {
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct CloudConfigView {
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spki_pins: Vec<String>,
    pub heartbeat_s: u64,
    pub backoff_min_s: u64,
    pub backoff_max_s: u64,
    pub queue_max_events: usize,
    pub queue_max_age_days: u32,
}

#[derive(Serialize)]
pub struct GpioConfigView {
    pub reed_in: u8,
    pub reed_active_low: bool,
    pub siren_out: u8,
    pub floodlight_out: u8,
    pub radio433_rx_in: u8,
    pub debounce_ms: u64,
}

#[derive(Serialize)]
pub struct TimerConfigView {
    pub exit_delay_s: u64,
    pub entry_delay_s: u64,
    pub auto_rearm_s: u64,
    pub siren_max_s: u64,
}

#[derive(Serialize)]
pub struct BleConfigView {
    pub enabled: bool,
    pub pairing_window_s: u64,
}

#[derive(Serialize)]
pub struct Rf433ConfigView {
    pub enabled: bool,
    pub allow_disarm: bool,
    pub debounce_ms: u64,
}

#[derive(Deserialize)]
pub struct ConfigUpdateRequest {
    #[serde(flatten)]
    config: Value,
}

/// GET /v1/config - Get current configuration snapshot
pub async fn get_config(
    State(ctx): State<Arc<ApiContext>>,
) -> Result<Json<ConfigResponse>, ApiError> {
    let config = &ctx.config;

    let response = ConfigResponse {
        system: SystemConfigView {
            client_id: config.system.client_id.clone(),
            data_dir: config.system.data_dir.display().to_string(),
            log_level: config.system.log_level.clone(),
        },
        network: NetworkConfigView {
            prefer: config.network.prefer.clone(),
            enable_lte: config.network.enable_lte,
        },
        http: HttpConfigView {
            listen_addr: config.http.listen_addr.clone(),
        },
        ws_local: WsLocalConfigView {
            enabled: config.ws_local.enabled,
        },
        cloud: CloudConfigView {
            url: config.cloud.url.clone(),
            spki_pins: config.cloud.spki_pins.clone(),
            heartbeat_s: config.cloud.heartbeat_s,
            backoff_min_s: config.cloud.backoff_min_s,
            backoff_max_s: config.cloud.backoff_max_s,
            queue_max_events: config.cloud.queue_max_events,
            queue_max_age_days: config.cloud.queue_max_age_days,
        },
        gpio: GpioConfigView {
            reed_in: config.gpio.reed_in,
            reed_active_low: config.gpio.reed_active_low,
            siren_out: config.gpio.siren_out,
            floodlight_out: config.gpio.floodlight_out,
            radio433_rx_in: config.gpio.radio433_rx_in,
            debounce_ms: config.gpio.debounce_ms,
        },
        timers: TimerConfigView {
            exit_delay_s: config.timers.exit_delay_s,
            entry_delay_s: config.timers.entry_delay_s,
            auto_rearm_s: config.timers.auto_rearm_s,
            siren_max_s: config.timers.siren_max_s,
        },
        ble: BleConfigView {
            enabled: config.ble.enabled,
            pairing_window_s: config.ble.pairing_window_s,
        },
        rf433: Rf433ConfigView {
            enabled: config.rf433.enabled,
            allow_disarm: config.rf433.allow_disarm,
            debounce_ms: config.rf433.debounce_ms,
        },
    };

    Ok(Json(response))
}

/// PUT /v1/config - Update configuration (requires restart)
pub async fn update_config(
    State(_ctx): State<Arc<ApiContext>>,
    Json(request): Json<ConfigUpdateRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // Validate the configuration update
    // In a real implementation, this would:
    // 1. Validate the configuration against schema
    // 2. Write to disk at /etc/pi-door-client/config.toml
    // 3. Mark restart as required
    // 4. Optionally trigger SIGHUP for hot-reload of certain configs

    // For now, just validate it's valid JSON
    if request.config.is_null() {
        return Err(ApiError {
            message: "Configuration cannot be null".to_string(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "applied": false,
            "restart_required": true,
            "message": "Configuration update received. Restart required to apply changes.",
            "note": "Configuration persistence requires write access to /etc/pi-door-client/config.toml"
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::events::EventBus;
    use crate::state::new_app_state;

    #[tokio::test]
    async fn test_get_config() {
        let state = new_app_state();
        let (event_bus, _) = EventBus::new();
        let config = AppConfig::test_default();
        let ctx = Arc::new(ApiContext {
            state,
            event_bus,
            config,
        });

        let result = get_config(State(ctx)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.system.client_id.is_empty());
        assert_eq!(response.system.client_id, "test-client");
        assert_eq!(response.timers.exit_delay_s, 30);
    }

    #[tokio::test]
    async fn test_update_config() {
        let state = new_app_state();
        let (event_bus, _) = EventBus::new();
        let config = AppConfig::test_default();
        let ctx = Arc::new(ApiContext {
            state,
            event_bus,
            config,
        });

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
