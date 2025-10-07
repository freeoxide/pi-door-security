//! Shared state structures

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::events::EventEnvelope;

/// Main alarm state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlarmState {
    Disarmed,
    ExitDelay,
    Armed,
    EntryDelay,
    Alarm,
}

impl std::fmt::Display for AlarmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlarmState::Disarmed => write!(f, "disarmed"),
            AlarmState::ExitDelay => write!(f, "exit_delay"),
            AlarmState::Armed => write!(f, "armed"),
            AlarmState::EntryDelay => write!(f, "entry_delay"),
            AlarmState::Alarm => write!(f, "alarm"),
        }
    }
}

/// Actuator state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActuatorState {
    pub siren: bool,
    pub floodlight: bool,
}

impl Default for ActuatorState {
    fn default() -> Self {
        Self {
            siren: false,
            floodlight: false,
        }
    }
}

/// Connectivity state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectivityState {
    pub cloud: CloudStatus,
    pub interface: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudStatus {
    Online,
    Offline,
    Connecting,
}

impl Default for ConnectivityState {
    fn default() -> Self {
        Self {
            cloud: CloudStatus::Offline,
            interface: None,
        }
    }
}

/// Timer state tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimerState {
    /// Remaining seconds for exit delay (0 if not active)
    pub exit_s: u64,
    /// Remaining seconds for entry delay (0 if not active)
    pub entry_s: u64,
    /// Remaining seconds for auto-rearm (0 if not active)
    pub auto_rearm_s: u64,
    /// Remaining seconds for siren (0 if not active)
    pub siren_s: u64,
}

/// Shared application state
#[derive(Debug, Clone)]
pub struct SharedState {
    /// Current alarm state
    pub alarm_state: AlarmState,
    /// Door sensor state (true = open)
    pub door_open: bool,
    /// Actuator states
    pub actuators: ActuatorState,
    /// Connectivity state
    pub connectivity: ConnectivityState,
    /// Active timer state
    pub timers: TimerState,
    /// Recent events (limited to last 50)
    pub last_events: VecDeque<EventEnvelope>,
    /// When the state was last updated
    pub last_updated: DateTime<Utc>,
    /// Application start time
    pub start_time: DateTime<Utc>,
}

impl Default for SharedState {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            alarm_state: AlarmState::Disarmed,
            door_open: false,
            actuators: ActuatorState::default(),
            connectivity: ConnectivityState::default(),
            timers: TimerState::default(),
            last_events: VecDeque::with_capacity(50),
            last_updated: now,
            start_time: now,
        }
    }
}

impl SharedState {
    /// Create new shared state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to the recent events list
    pub fn add_event(&mut self, event: EventEnvelope) {
        if self.last_events.len() >= 50 {
            self.last_events.pop_front();
        }
        self.last_events.push_back(event);
        self.last_updated = Utc::now();
    }

    /// Get uptime in seconds
    pub fn uptime_s(&self) -> i64 {
        (Utc::now() - self.start_time).num_seconds()
    }

    /// Set alarm state and update timestamp
    pub fn set_alarm_state(&mut self, state: AlarmState) {
        self.alarm_state = state;
        self.last_updated = Utc::now();
    }

    /// Set door state and update timestamp
    pub fn set_door_state(&mut self, open: bool) {
        self.door_open = open;
        self.last_updated = Utc::now();
    }

    /// Set actuator state and update timestamp
    pub fn set_actuators(&mut self, actuators: ActuatorState) {
        self.actuators = actuators;
        self.last_updated = Utc::now();
    }

    /// Set connectivity state and update timestamp
    pub fn set_connectivity(&mut self, connectivity: ConnectivityState) {
        self.connectivity = connectivity;
        self.last_updated = Utc::now();
    }
}

/// Thread-safe shared application state
pub type AppState = Arc<RwLock<SharedState>>;

/// Create a new AppState
pub fn new_app_state() -> AppState {
    Arc::new(RwLock::new(SharedState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alarm_state_display() {
        assert_eq!(AlarmState::Disarmed.to_string(), "disarmed");
        assert_eq!(AlarmState::Armed.to_string(), "armed");
        assert_eq!(AlarmState::Alarm.to_string(), "alarm");
    }

    #[test]
    fn test_shared_state_add_event() {
        let mut state = SharedState::new();
        
        for i in 0..60 {
            let envelope = EventEnvelope::new(
                crate::events::Event::DoorOpen,
                "test".to_string()
            );
            state.add_event(envelope);
        }

        // Should cap at 50
        assert_eq!(state.last_events.len(), 50);
    }

    #[test]
    fn test_app_state_thread_safety() {
        let state = new_app_state();
        
        {
            let mut s = state.write();
            s.set_alarm_state(AlarmState::Armed);
        }
        
        {
            let s = state.read();
            assert_eq!(s.alarm_state, AlarmState::Armed);
        }
    }

    #[test]
    fn test_uptime_calculation() {
        let state = SharedState::new();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(state.uptime_s() >= 0);
    }
}
