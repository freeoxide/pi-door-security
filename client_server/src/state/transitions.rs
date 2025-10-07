//! State transition rules and logic

use super::{AlarmState, ActuatorState};
use crate::events::Event;
use tracing::debug;

/// Represents a state transition
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransition {
    pub from: AlarmState,
    pub to: AlarmState,
    pub event: String,
}

impl StateTransition {
    pub fn new(from: AlarmState, to: AlarmState, event: String) -> Self {
        Self { from, to, event }
    }
}

/// Determine the next state based on current state and event
pub fn next_state(current: AlarmState, event: &Event) -> Option<AlarmState> {
    let next = match (current, event) {
        // User arm from disarmed -> exit delay
        (AlarmState::Disarmed, Event::UserArm { .. }) => Some(AlarmState::ExitDelay),
        
        // Exit delay expired -> armed
        (AlarmState::ExitDelay, Event::TimerExitExpired) => Some(AlarmState::Armed),
        
        // User disarm from exit delay -> disarmed
        (AlarmState::ExitDelay, Event::UserDisarm { .. }) => Some(AlarmState::Disarmed),
        
        // Door open while armed -> entry delay
        (AlarmState::Armed, Event::DoorOpen) => Some(AlarmState::EntryDelay),
        
        // User disarm from armed -> disarmed
        (AlarmState::Armed, Event::UserDisarm { .. }) => Some(AlarmState::Disarmed),
        
        // Entry delay expired -> alarm
        (AlarmState::EntryDelay, Event::TimerEntryExpired) => Some(AlarmState::Alarm),
        
        // User disarm from entry delay -> disarmed
        (AlarmState::EntryDelay, Event::UserDisarm { .. }) => Some(AlarmState::Disarmed),
        
        // User disarm from alarm -> disarmed
        (AlarmState::Alarm, Event::UserDisarm { .. }) => Some(AlarmState::Disarmed),
        
        // Auto-rearm from alarm -> exit delay (then armed)
        (AlarmState::Alarm, Event::TimerAutoRearmExpired) => Some(AlarmState::ExitDelay),
        
        // Auto-rearm from disarmed -> exit delay (then armed)
        (AlarmState::Disarmed, Event::TimerAutoRearmExpired) => Some(AlarmState::ExitDelay),
        
        // No transition for other combinations
        _ => None,
    };

    if let Some(new_state) = next {
        if new_state != current {
            debug!(
                from = %current,
                to = %new_state,
                ?event,
                "State transition"
            );
        }
    }

    next
}

/// Determine actuator state based on alarm state
pub fn actuator_state_for(alarm_state: AlarmState, in_alarm: bool) -> ActuatorState {
    match alarm_state {
        AlarmState::Alarm => ActuatorState {
            siren: in_alarm, // Siren on only if we're in active alarm
            floodlight: true,
        },
        _ => ActuatorState {
            siren: false,
            floodlight: false,
        },
    }
}

/// Check if a transition is valid
pub fn is_valid_transition(from: AlarmState, to: AlarmState, event: &Event) -> bool {
    next_state(from, event) == Some(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;

    #[test]
    fn test_disarmed_to_exit_delay() {
        let event = Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(30),
        };
        assert_eq!(
            next_state(AlarmState::Disarmed, &event),
            Some(AlarmState::ExitDelay)
        );
    }

    #[test]
    fn test_exit_delay_to_armed() {
        let event = Event::TimerExitExpired;
        assert_eq!(
            next_state(AlarmState::ExitDelay, &event),
            Some(AlarmState::Armed)
        );
    }

    #[test]
    fn test_armed_to_entry_delay_on_door_open() {
        let event = Event::DoorOpen;
        assert_eq!(
            next_state(AlarmState::Armed, &event),
            Some(AlarmState::EntryDelay)
        );
    }

    #[test]
    fn test_entry_delay_to_alarm() {
        let event = Event::TimerEntryExpired;
        assert_eq!(
            next_state(AlarmState::EntryDelay, &event),
            Some(AlarmState::Alarm)
        );
    }

    #[test]
    fn test_disarm_from_any_state() {
        let event = Event::UserDisarm {
            source: EventSource::Local,
            auto_rearm_s: None,
        };
        
        assert_eq!(
            next_state(AlarmState::ExitDelay, &event),
            Some(AlarmState::Disarmed)
        );
        assert_eq!(
            next_state(AlarmState::Armed, &event),
            Some(AlarmState::Disarmed)
        );
        assert_eq!(
            next_state(AlarmState::EntryDelay, &event),
            Some(AlarmState::Disarmed)
        );
        assert_eq!(
            next_state(AlarmState::Alarm, &event),
            Some(AlarmState::Disarmed)
        );
    }

    #[test]
    fn test_auto_rearm() {
        let event = Event::TimerAutoRearmExpired;
        assert_eq!(
            next_state(AlarmState::Disarmed, &event),
            Some(AlarmState::ExitDelay)
        );
        assert_eq!(
            next_state(AlarmState::Alarm, &event),
            Some(AlarmState::ExitDelay)
        );
    }

    #[test]
    fn test_door_close_doesnt_affect_entry_delay() {
        let event = Event::DoorClose;
        assert_eq!(next_state(AlarmState::EntryDelay, &event), None);
    }

    #[test]
    fn test_actuator_states() {
        assert_eq!(
            actuator_state_for(AlarmState::Disarmed, false),
            ActuatorState { siren: false, floodlight: false }
        );
        
        assert_eq!(
            actuator_state_for(AlarmState::Alarm, true),
            ActuatorState { siren: true, floodlight: true }
        );
        
        assert_eq!(
            actuator_state_for(AlarmState::Alarm, false), // Siren timer expired
            ActuatorState { siren: false, floodlight: true }
        );
    }

    #[test]
    fn test_is_valid_transition() {
        let event = Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(30),
        };
        
        assert!(is_valid_transition(
            AlarmState::Disarmed,
            AlarmState::ExitDelay,
            &event
        ));
        
        assert!(!is_valid_transition(
            AlarmState::Disarmed,
            AlarmState::Armed,
            &event
        ));
    }
}
