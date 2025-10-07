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
    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Disarmed);

    // Wait and verify alarm doesn't trigger
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Disarmed);
    assert!(!state.read().actuators.siren);
}

#[tokio::test]
async fn test_auto_rearm() {
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

    // Disarm with auto-rearm
    event_bus
        .emit(Event::UserDisarm {
            source: EventSource::Local,
            auto_rearm_s: Some(3),
        })
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Disarmed);

    // Wait for auto-rearm
    sleep(Duration::from_secs(4)).await;
    assert_eq!(state.read().alarm_state, AlarmState::ExitDelay);

    // Wait for exit delay
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Armed);
}

#[tokio::test]
async fn test_siren_timer_expiration() {
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

    // Trigger alarm
    event_bus
        .emit(Event::UserArm {
            source: EventSource::Local,
            exit_delay_s: Some(1),
        })
        .unwrap();
    sleep(Duration::from_secs(2)).await;

    event_bus.emit(Event::DoorOpen).unwrap();
    sleep(Duration::from_secs(3)).await;

    // Verify alarm is active
    assert_eq!(state.read().alarm_state, AlarmState::Alarm);
    assert!(state.read().actuators.siren);

    // Wait for siren timer to expire
    sleep(Duration::from_secs(3)).await;
    assert!(!state.read().actuators.siren); // Siren should be off
    assert!(state.read().actuators.floodlight); // Floodlight still on
}

#[tokio::test]
async fn test_manual_actuator_control() {
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

    // Turn on siren manually
    event_bus
        .emit(Event::SirenControl {
            on: true,
            duration_s: Some(2),
        })
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    assert!(state.read().actuators.siren);

    // Wait for timer
    sleep(Duration::from_secs(3)).await;
    assert!(!state.read().actuators.siren);

    // Turn on floodlight manually
    event_bus
        .emit(Event::FloodlightControl {
            on: true,
            duration_s: Some(2),
        })
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    assert!(state.read().actuators.floodlight);

    // Turn off manually
    event_bus
        .emit(Event::FloodlightControl {
            on: false,
            duration_s: None,
        })
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    assert!(!state.read().actuators.floodlight);
}

#[tokio::test]
async fn test_door_close_during_entry_delay_doesnt_cancel() {
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
            exit_delay_s: Some(1),
        })
        .unwrap();
    sleep(Duration::from_secs(2)).await;

    // Open door
    event_bus.emit(Event::DoorOpen).unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::EntryDelay);

    // Close door - should NOT cancel entry delay
    event_bus.emit(Event::DoorClose).unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(state.read().alarm_state, AlarmState::EntryDelay);
    assert!(!state.read().door_open);

    // Wait for entry delay - should still trigger alarm
    sleep(Duration::from_secs(3)).await;
    assert_eq!(state.read().alarm_state, AlarmState::Alarm);
}
