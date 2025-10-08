//! HTTP and WebSocket API integration tests

use pi_door_client::{
    api,
    config::AppConfig,
    events::EventBus,
    state::{new_app_state, StateMachine},
};
use reqwest;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let state = new_app_state();
    let (event_bus, mut event_rx) = EventBus::new();
    let config = AppConfig::test_default();
    
    // Spawn state machine to process events
    let mut state_machine = StateMachine::new(
        state.clone(),
        event_bus.clone(),
        config.timers.clone(),
        config.system.client_id.clone(),
    );
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let _ = state_machine.process_event(event).await;
        }
    });
    
    let app = api::create_router(state, event_bus, config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);
    
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    (url, handle)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (url, handle) = start_test_server().await;
    
    let response = reqwest::get(format!("{}/v1/health", url))
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["ready"], true);
    assert_eq!(json["version"], "0.1.0");
    
    handle.abort();
}

#[tokio::test]
async fn test_status_endpoint() {
    let (url, handle) = start_test_server().await;
    
    let response = reqwest::get(format!("{}/v1/status", url))
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["state"], "disarmed");
    assert_eq!(json["door"], "closed");
    assert_eq!(json["actuators"]["siren"], false);
    assert_eq!(json["actuators"]["floodlight"], false);
    
    handle.abort();
}

#[tokio::test]
async fn test_arm_endpoint() {
    let (url, handle) = start_test_server().await;
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/arm", url))
        .json(&json!({"exit_delay_s": 30}))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 202);
    
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["state"], "exit_delay");
    assert_eq!(json["exit_delay_s"], 30);
    
    handle.abort();
}

#[tokio::test]
async fn test_disarm_endpoint() {
    let (url, handle) = start_test_server().await;
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/disarm", url))
        .json(&json!({"auto_rearm_s": 120}))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 202);
    
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["state"], "disarmed");
    assert_eq!(json["auto_rearm_s"], 120);
    
    handle.abort();
}

#[tokio::test]
async fn test_siren_control_endpoint() {
    let (url, handle) = start_test_server().await;
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/siren", url))
        .json(&json!({"on": true, "duration_s": 60}))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 202);
    
    let json: serde_json::Value = response.json().await.unwrap();
    assert!(json["duration_s"].is_number());
    
    handle.abort();
}

#[tokio::test]
async fn test_floodlight_control_endpoint() {
    let (url, handle) = start_test_server().await;
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/floodlight", url))
        .json(&json!({"on": true, "duration_s": 600}))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 202);
    
    handle.abort();
}

#[tokio::test]
async fn test_full_arm_disarm_workflow() {
    let (url, handle) = start_test_server().await;
    let client = reqwest::Client::new();
    
    // Check initial status
    let response = client.get(format!("{}/v1/status", url)).send().await.unwrap();
    let status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(status["state"], "disarmed");
    
    // Arm the system
    let response = client
        .post(format!("{}/v1/arm", url))
        .json(&json!({"exit_delay_s": 5}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    
    // Wait a moment for state to update
    sleep(Duration::from_millis(200)).await;
    
    // Check status shows exit_delay
    let response = client.get(format!("{}/v1/status", url)).send().await.unwrap();
    let status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(status["state"], "exit_delay");
    
    // Disarm
    let response = client
        .post(format!("{}/v1/disarm", url))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    
    // Wait and check back to disarmed
    sleep(Duration::from_millis(200)).await;
    let response = client.get(format!("{}/v1/status", url)).send().await.unwrap();
    let status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(status["state"], "disarmed");
    
    handle.abort();
}
