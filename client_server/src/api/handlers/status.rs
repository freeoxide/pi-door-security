//! Status endpoint handler

use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

use crate::api::ApiContext;
use crate::state::AlarmState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub state: String,
    pub door: String,
    pub timers: TimersStatus,
    pub actuators: ActuatorsStatus,
    pub connectivity: ConnectivityStatus,
    pub last_events: Vec<Value>,
}

#[derive(Serialize)]
pub struct TimersStatus {
    pub exit_s: u64,
    pub entry_s: u64,
    pub auto_rearm_s: u64,
}

#[derive(Serialize)]
pub struct ActuatorsStatus {
    pub siren: bool,
    pub floodlight: bool,
}

#[derive(Serialize)]
pub struct ConnectivityStatus {
    pub cloud: String,
    pub iface: Option<String>,
}

/// GET /v1/status - Get current system status
pub async fn get_status(
    State(ctx): State<Arc<ApiContext>>,
) -> Json<StatusResponse> {
    let state = ctx.state.read();
    
    let alarm_state = match state.alarm_state {
        AlarmState::Disarmed => "disarmed",
        AlarmState::ExitDelay => "exit_delay",
        AlarmState::Armed => "armed",
        AlarmState::EntryDelay => "entry_delay",
        AlarmState::Alarm => "alarm",
    };
    
    let door_state = if state.door_open { "open" } else { "closed" };
    
    let cloud_status = match state.connectivity.cloud {
        crate::state::CloudStatus::Online => "online",
        crate::state::CloudStatus::Offline => "offline",
        crate::state::CloudStatus::Connecting => "connecting",
    };
    
    // Convert last events to JSON
    let last_events: Vec<Value> = state.last_events
        .iter()
        .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
        .collect();
    
    Json(StatusResponse {
        state: alarm_state.to_string(),
        door: door_state.to_string(),
        timers: TimersStatus {
            exit_s: state.timers.exit_s,
            entry_s: state.timers.entry_s,
            auto_rearm_s: state.timers.auto_rearm_s,
        },
        actuators: ActuatorsStatus {
            siren: state.actuators.siren,
            floodlight: state.actuators.floodlight,
        },
        connectivity: ConnectivityStatus {
            cloud: cloud_status.to_string(),
            iface: state.connectivity.interface.clone(),
        },
        last_events,
    })
}
