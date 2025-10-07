//! Comprehensive state machine integration tests

use pi_door_client::{
    config::TimerConfig,
    events::{Event, EventBus, EventSource},
    state::{new_app_state, AlarmState, StateMachine},
};
use tokio::time::{sleep, Duration};

fn test_timer_config() -> TimerConfig {
    TimerConfig {
        exit_delay_s: 2,
        entry_delay_s: 2,
        auto_rearm_s: 3,
        siren_max_s: 2,
    }
}

#[tokio::test]
async fn test_complete_arm_cycle() {
    let state = new_app_state();
    let (event_bus, mut event_rx) = EventBus::new();
    let mut sm = StateMachine::new(
        state.clone(),
        event_bus.clone(),
        test_timer_config(),
        "test".to_string(),
    );

    // Spawn event processor
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let _ = sm.process_event(event).await;
        }
    });

    // Initial state: disarmed
    assert_eq!(state.read().alarm_state, AlarmState::Disarmed);

    // Arm the system
    event_bus
        .emit(Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(2),
        })
        .unwrap();

    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::ExitDelay);

    // Wait for exit delay to expire
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Armed);
}

#[tokio::test]
async fn test_alarm_trigger_on_door_open() {
    let state = new_app_state();
    let (event_bus, mut event_rx) = EventBus::new();
    let mut sm = StateMachine::new(
        state.clone(),
        event_bus.clone(),
        test_timer_config(),
        "test".to_string(),
    );

    // Spawn event processor
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let _ = sm.process_event(event).await;
        }
    });

    // Arm and wait to become fully armed
    event_bus
        .emit(Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(2),
        })
        .unwrap();
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Armed);

    // Open door - should trigger entry delay
    event_bus.emit(Event::DoorOpen).unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::EntryDelay);
    assert!(state.read().door_open);

    // Wait for entry delay to expire - should trigger alarm
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Alarm);
    assert!(state.read().actuators.siren);
    assert!(state.read().actuators.floodlight);
}

#[tokio::test]
async fn test_disarm_during_entry_delay() {
    let state = new_app_state();
    let (event_bus, mut event_rx) = EventBus::new();
    let mut sm = StateMachine::new(
        state.clone(),
        event_bus.clone(),
        test_timer_config(),
        "test".to_string(),
    );

    // Spawn event processor
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let _ = sm.process_event(event).await;
        }
    });

    // Arm system
    event_bus
        .emit(Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(2),
        })
        .unwrap();
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Armed);

    // Open door
    event_bus.emit(Event::DoorOpen).unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::EntryDelay);

    // Disarm before entry delay expires
    event_bus
        .emit(Event::UserDisarm {
            source: EventSource::Local,
            auto_rearm_s: None,
        })
        .unwrap();
    sleep(Duration::from_millis(200)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Disarmed);
    assert!(!state.read().actuators.siren);

    // Wait longer and verify alarm still doesn't trigger
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Disarmed);
}
