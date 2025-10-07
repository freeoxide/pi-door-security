//! State machine implementation

use super::{AlarmState, AppState, ActuatorState, StateTransition};
use super::transitions::{next_state, actuator_state_for};
use crate::config::TimerConfig;
use crate::events::{Event, EventBus, EventEnvelope, TimerId};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// State machine that processes events and manages state transitions
pub struct StateMachine {
    /// Shared application state
    state: AppState,
    /// Event bus for emitting new events
    event_bus: EventBus,
    /// Timer configuration
    timer_config: TimerConfig,
    /// Client ID for event envelopes
    client_id: String,
    /// Timer handles
    timer_tx: mpsc::UnboundedSender<TimerCommand>,
}

/// Commands for timer management
#[derive(Debug)]
enum TimerCommand {
    Start { id: TimerId, duration_s: u64 },
    Cancel { id: TimerId },
    CancelAll,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new(
        state: AppState,
        event_bus: EventBus,
        timer_config: TimerConfig,
        client_id: String,
    ) -> Self {
        let (timer_tx, timer_rx) = mpsc::unbounded_channel();
        
        // Spawn timer manager task
        let bus_clone = event_bus.clone();
        tokio::spawn(async move {
            Self::timer_manager(timer_rx, bus_clone).await;
        });

        Self {
            state,
            event_bus,
            timer_config,
            client_id,
            timer_tx,
        }
    }

    /// Process an incoming event
    pub async fn process_event(&mut self, event: Event) -> Result<()> {
        debug!(?event, "Processing event");

        let current_state = {
            let state = self.state.read();
            state.alarm_state
        };

        // Handle the event based on current state
        match &event {
            Event::UserArm { exit_delay_s, .. } => {
                self.handle_user_arm(current_state, *exit_delay_s).await?;
            }
            Event::UserDisarm { auto_rearm_s, .. } => {
                self.handle_user_disarm(current_state, *auto_rearm_s).await?;
            }
            Event::DoorOpen => {
                self.handle_door_open(current_state).await?;
            }
            Event::DoorClose => {
                self.handle_door_close().await?;
            }
            Event::TimerExitExpired => {
                self.handle_timer_exit_expired(current_state).await?;
            }
            Event::TimerEntryExpired => {
                self.handle_timer_entry_expired(current_state).await?;
            }
            Event::TimerAutoRearmExpired => {
                self.handle_timer_auto_rearm_expired(current_state).await?;
            }
            Event::TimerSirenExpired => {
                self.handle_timer_siren_expired().await?;
            }
            Event::SirenControl { on, duration_s } => {
                self.handle_siren_control(*on, *duration_s).await?;
            }
            Event::FloodlightControl { on, duration_s } => {
                self.handle_floodlight_control(*on, *duration_s).await?;
            }
            _ => {
                debug!(?event, "Event does not require state machine action");
            }
        }

        // Create and store event envelope
        let envelope = EventEnvelope::new(event, self.client_id.clone());
        {
            let mut state = self.state.write();
            state.add_event(envelope.clone());
        }

        // Broadcast to subscribers
        self.event_bus.broadcast(envelope)?;

        Ok(())
    }

    async fn handle_user_arm(&mut self, current_state: AlarmState, exit_delay_s: Option<u64>) -> Result<()> {
        if let Some(new_state) = next_state(current_state, &Event::UserArm { 
            source: crate::events::EventSource::System,
            exit_delay_s 
        }) {
            self.transition_to(new_state).await?;
            
            // Start exit delay timer
            let delay = exit_delay_s.unwrap_or(self.timer_config.exit_delay_s);
            self.start_timer(TimerId::ExitDelay, delay)?;
            
            info!(exit_delay_s = delay, "System arming with exit delay");
        }
        Ok(())
    }

    async fn handle_user_disarm(&mut self, current_state: AlarmState, auto_rearm_s: Option<u64>) -> Result<()> {
        if let Some(new_state) = next_state(current_state, &Event::UserDisarm {
            source: crate::events::EventSource::System,
            auto_rearm_s
        }) {
            // Cancel all timers
            self.cancel_all_timers()?;
            
            self.transition_to(new_state).await?;
            
            // Set actuators to off
            {
                let mut state = self.state.write();
                state.set_actuators(ActuatorState {
                    siren: false,
                    floodlight: false,
                });
            }
            
            // Start auto-rearm timer if configured
            let auto_rearm = auto_rearm_s.unwrap_or(self.timer_config.auto_rearm_s);
            if auto_rearm > 0 {
                self.start_timer(TimerId::AutoRearm, auto_rearm)?;
                info!(auto_rearm_s = auto_rearm, "System disarmed with auto-rearm");
            } else {
                info!("System disarmed");
            }
        }
        Ok(())
    }

    async fn handle_door_open(&mut self, current_state: AlarmState) -> Result<()> {
        {
            let mut state = self.state.write();
            state.set_door_state(true);
        }

        if let Some(new_state) = next_state(current_state, &Event::DoorOpen) {
            self.transition_to(new_state).await?;
            
            // Start entry delay timer
            self.start_timer(TimerId::EntryDelay, self.timer_config.entry_delay_s)?;
            
            warn!(entry_delay_s = self.timer_config.entry_delay_s, "Door opened while armed - entry delay started");
        } else {
            debug!("Door opened (no state change)");
        }
        
        Ok(())
    }

    async fn handle_door_close(&mut self) -> Result<()> {
        {
            let mut state = self.state.write();
            state.set_door_state(false);
        }
        debug!("Door closed");
        Ok(())
    }

    async fn handle_timer_exit_expired(&mut self, current_state: AlarmState) -> Result<()> {
        if let Some(new_state) = next_state(current_state, &Event::TimerExitExpired) {
            self.transition_to(new_state).await?;
            info!("Exit delay expired - system now armed");
        }
        Ok(())
    }

    async fn handle_timer_entry_expired(&mut self, current_state: AlarmState) -> Result<()> {
        if let Some(new_state) = next_state(current_state, &Event::TimerEntryExpired) {
            self.transition_to(new_state).await?;
            
            // Activate alarm
            {
                let mut state = self.state.write();
                state.set_actuators(ActuatorState {
                    siren: true,
                    floodlight: true,
                });
            }
            
            // Start siren timer
            self.start_timer(TimerId::Siren, self.timer_config.siren_max_s)?;
            
            warn!("ALARM TRIGGERED - entry delay expired");
        }
        Ok(())
    }

    async fn handle_timer_auto_rearm_expired(&mut self, current_state: AlarmState) -> Result<()> {
        if let Some(new_state) = next_state(current_state, &Event::TimerAutoRearmExpired) {
            self.transition_to(new_state).await?;
            
            // Start exit delay
            self.start_timer(TimerId::ExitDelay, self.timer_config.exit_delay_s)?;
            
            info!("Auto-rearm triggered - starting exit delay");
        }
        Ok(())
    }

    async fn handle_timer_siren_expired(&mut self) -> Result<()> {
        {
            let mut state = self.state.write();
            let mut actuators = state.actuators;
            actuators.siren = false;
            state.set_actuators(actuators);
        }
        info!("Siren timer expired - siren off");
        Ok(())
    }

    async fn handle_siren_control(&mut self, on: bool, duration_s: Option<u64>) -> Result<()> {
        {
            let mut state = self.state.write();
            let mut actuators = state.actuators;
            actuators.siren = on;
            state.set_actuators(actuators);
        }

        if on {
            if let Some(duration) = duration_s {
                self.start_timer(TimerId::Siren, duration)?;
            }
            info!(duration_s, "Siren manually activated");
        } else {
            self.cancel_timer(TimerId::Siren)?;
            info!("Siren manually deactivated");
        }

        Ok(())
    }

    async fn handle_floodlight_control(&mut self, on: bool, duration_s: Option<u64>) -> Result<()> {
        {
            let mut state = self.state.write();
            let mut actuators = state.actuators;
            actuators.floodlight = on;
            state.set_actuators(actuators);
        }

        if on {
            if let Some(duration) = duration_s {
                self.start_timer(TimerId::Floodlight, duration)?;
            }
            info!(duration_s, "Floodlight manually activated");
        } else {
            self.cancel_timer(TimerId::Floodlight)?;
            info!("Floodlight manually deactivated");
        }

        Ok(())
    }

    async fn transition_to(&mut self, new_state: AlarmState) -> Result<()> {
        let old_state = {
            let mut state = self.state.write();
            let old = state.alarm_state;
            state.set_alarm_state(new_state);
            old
        };

        info!(from = %old_state, to = %new_state, "State transition");
        
        Ok(())
    }

    fn start_timer(&self, id: TimerId, duration_s: u64) -> Result<()> {
        self.timer_tx.send(TimerCommand::Start { id, duration_s })?;
        debug!(?id, duration_s, "Timer started");
        Ok(())
    }

    fn cancel_timer(&self, id: TimerId) -> Result<()> {
        self.timer_tx.send(TimerCommand::Cancel { id })?;
        debug!(?id, "Timer cancelled");
        Ok(())
    }

    fn cancel_all_timers(&self) -> Result<()> {
        self.timer_tx.send(TimerCommand::CancelAll)?;
        debug!("All timers cancelled");
        Ok(())
    }

    /// Timer manager task
    async fn timer_manager(
        mut rx: mpsc::UnboundedReceiver<TimerCommand>,
        event_bus: EventBus,
    ) {
        use std::collections::HashMap;
        use tokio::task::JoinHandle;

        let mut handles: HashMap<TimerId, JoinHandle<()>> = HashMap::new();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                TimerCommand::Start { id, duration_s } => {
                    // Cancel existing timer if any
                    if let Some(handle) = handles.remove(&id) {
                        handle.abort();
                    }

                    // Start new timer
                    let bus = event_bus.clone();
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(duration_s)).await;
                        
                        let event = match id {
                            TimerId::ExitDelay => Event::TimerExitExpired,
                            TimerId::EntryDelay => Event::TimerEntryExpired,
                            TimerId::AutoRearm => Event::TimerAutoRearmExpired,
                            TimerId::Siren => Event::TimerSirenExpired,
                            TimerId::Floodlight => Event::FloodlightControl { on: false, duration_s: None },
                        };

                        let _ = bus.emit(event);
                    });

                    handles.insert(id, handle);
                }
                TimerCommand::Cancel { id } => {
                    if let Some(handle) = handles.remove(&id) {
                        handle.abort();
                    }
                }
                TimerCommand::CancelAll => {
                    for (_, handle) in handles.drain() {
                        handle.abort();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::new_app_state;

    fn test_config() -> TimerConfig {
        TimerConfig {
            exit_delay_s: 5,
            entry_delay_s: 5,
            auto_rearm_s: 10,
            siren_max_s: 10,
        }
    }

    #[tokio::test]
    async fn test_arm_disarm_cycle() {
        let state = new_app_state();
        let (bus, _rx) = EventBus::new();
        let mut sm = StateMachine::new(
            state.clone(),
            bus.clone(),
            test_config(),
            "test".to_string(),
        );

        // Initial state should be disarmed
        assert_eq!(state.read().alarm_state, AlarmState::Disarmed);

        // Arm the system
        sm.process_event(Event::UserArm {
            source: crate::events::EventSource::Local,
            exit_delay_s: Some(5),
        }).await.unwrap();

        assert_eq!(state.read().alarm_state, AlarmState::ExitDelay);

        // Disarm before exit delay expires
        sm.process_event(Event::UserDisarm {
            source: crate::events::EventSource::Local,
            auto_rearm_s: None,
        }).await.unwrap();

        assert_eq!(state.read().alarm_state, AlarmState::Disarmed);
    }

    #[tokio::test]
    async fn test_door_open_triggers_entry_delay() {
        let state = new_app_state();
        let (bus, _rx) = EventBus::new();
        let mut sm = StateMachine::new(
            state.clone(),
            bus.clone(),
            test_config(),
            "test".to_string(),
        );

        // Arm system
        sm.process_event(Event::UserArm {
            source: crate::events::EventSource::Local,
            exit_delay_s: Some(5),
        }).await.unwrap();

        // Complete exit delay
        sm.process_event(Event::TimerExitExpired).await.unwrap();
        assert_eq!(state.read().alarm_state, AlarmState::Armed);

        // Open door
        sm.process_event(Event::DoorOpen).await.unwrap();
        assert_eq!(state.read().alarm_state, AlarmState::EntryDelay);
        assert!(state.read().door_open);
    }
}
