//! Event type definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Source of an event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventSource {
    Local,
    Ws,
    Cloud,
    Ble,
    Rf,
    System,
}

/// Main event type that drives the state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// User initiated arm command
    UserArm {
        source: EventSource,
        exit_delay_s: Option<u64>,
    },
    
    /// User initiated disarm command
    UserDisarm {
        source: EventSource,
        auto_rearm_s: Option<u64>,
    },
    
    /// Door opened
    DoorOpen,
    
    /// Door closed
    DoorClose,
    
    /// Exit delay timer expired
    TimerExitExpired,
    
    /// Entry delay timer expired
    TimerEntryExpired,
    
    /// Auto-rearm timer expired
    TimerAutoRearmExpired,
    
    /// Siren timer expired
    TimerSirenExpired,
    
    /// Cloud connectivity restored
    ConnectivityOnline,
    
    /// Cloud connectivity lost
    ConnectivityOffline,
    
    /// Manual siren control
    SirenControl {
        on: bool,
        duration_s: Option<u64>,
    },
    
    /// Manual floodlight control
    FloodlightControl {
        on: bool,
        duration_s: Option<u64>,
    },
    
    /// RF code received
    RfCodeReceived {
        code: String,
    },
}

/// Event with metadata for transmission and persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event: Event,
    pub client_id: String,
}

impl EventEnvelope {
    /// Create a new event envelope
    pub fn new(event: Event, client_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event,
            client_id,
        }
    }
}

/// Timer identifier for timer management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimerId {
    ExitDelay,
    EntryDelay,
    AutoRearm,
    Siren,
    Floodlight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(30),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("user_arm"));
        
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::UserArm { source, exit_delay_s } => {
                assert_eq!(source, EventSource::Local);
                assert_eq!(exit_delay_s, Some(30));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_envelope_creation() {
        let event = Event::DoorOpen;
        let envelope = EventEnvelope::new(event, "test-client".to_string());
        
        assert_eq!(envelope.client_id, "test-client");
        assert!(envelope.timestamp <= Utc::now());
    }
}
