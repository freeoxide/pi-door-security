//! WebSocket handler for real-time events

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::api::ApiContext;
use crate::events::{Event, EventSource};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMessage {
    Event {
        name: String,
        value: Option<String>,
        ts: String,
    },
    Cmd {
        name: String,
        #[serde(flatten)]
        args: serde_json::Value,
        id: String,
    },
    Ack {
        id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Ping,
    Pong,
}

/// GET /v1/ws - WebSocket upgrade endpoint
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<Arc<ApiContext>>,
) -> Response {
    info!("WebSocket connection request");
    ws.on_upgrade(move |socket| handle_socket(socket, ctx))
}

async fn handle_socket(socket: WebSocket, ctx: Arc<ApiContext>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to event bus
    let mut event_rx = ctx.event_bus.subscribe();
    
    // Spawn task to send events to client
    let mut send_task = tokio::spawn(async move {
        // Heartbeat interval (30 seconds)
        let mut heartbeat = interval(Duration::from_secs(30));
        
        loop {
            tokio::select! {
                // Send heartbeat ping
                _ = heartbeat.tick() => {
                    if sender.send(Message::Ping(vec![])).await.is_err() {
                        break;
                    }
                }
                
                // Forward events from event bus to WebSocket
                Ok(envelope) = event_rx.recv() => {
                    let ws_msg = match &envelope.event {
                        Event::UserArm { .. } => WsMessage::Event {
                            name: "state".to_string(),
                            value: Some("exit_delay".to_string()),
                            ts: envelope.timestamp.to_rfc3339(),
                        },
                        Event::UserDisarm { .. } => WsMessage::Event {
                            name: "state".to_string(),
                            value: Some("disarmed".to_string()),
                            ts: envelope.timestamp.to_rfc3339(),
                        },
                        Event::DoorOpen => WsMessage::Event {
                            name: "door".to_string(),
                            value: Some("open".to_string()),
                            ts: envelope.timestamp.to_rfc3339(),
                        },
                        Event::DoorClose => WsMessage::Event {
                            name: "door".to_string(),
                            value: Some("closed".to_string()),
                            ts: envelope.timestamp.to_rfc3339(),
                        },
                        Event::TimerEntryExpired => WsMessage::Event {
                            name: "alarm_triggered".to_string(),
                            value: None,
                            ts: envelope.timestamp.to_rfc3339(),
                        },
                        _ => continue, // Skip other events
                    };
                    
                    let json = match serde_json::to_string(&ws_msg) {
                        Ok(j) => j,
                        Err(e) => {
                            error!(error = %e, "Failed to serialize WebSocket message");
                            continue;
                        }
                    };
                    
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Spawn task to receive messages from client
    let event_bus = ctx.event_bus.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    debug!(text, "Received WebSocket message");
                    
                    // Parse command
                    let ws_msg: Result<WsMessage, _> = serde_json::from_str(&text);
                    match ws_msg {
                        Ok(WsMessage::Cmd { name, args, id }) => {
                            if let Err(e) = handle_command(&name, args, &event_bus) {
                                warn!(command = %name, error = %e, "Failed to handle command");
                            }
                        }
                        Ok(_) => {
                            debug!("Received non-command message");
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to parse WebSocket message");
                        }
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket connection closed by client");
                    break;
                }
                Message::Pong(_) => {
                    debug!("Received pong");
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    info!("WebSocket connection closed");
}

fn handle_command(
    name: &str,
    args: serde_json::Value,
    event_bus: &crate::events::EventBus,
) -> anyhow::Result<()> {
    let event = match name {
        "arm" => {
            let exit_delay = args.get("exit_delay_s")
                .and_then(|v| v.as_u64());
            Event::UserArm {
                source: EventSource::Ws,
                exit_delay_s: exit_delay,
            }
        }
        "disarm" => {
            let auto_rearm = args.get("auto_rearm_s")
                .and_then(|v| v.as_u64());
            Event::UserDisarm {
                source: EventSource::Ws,
                auto_rearm_s: auto_rearm,
            }
        }
        "siren" => {
            let on = args.get("on")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let duration = args.get("duration_s")
                .and_then(|v| v.as_u64());
            Event::SirenControl {
                on,
                duration_s: duration,
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown command: {}", name));
        }
    };

    event_bus.emit(event)?;
    info!(command = %name, "Command executed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Event {
            name: "door".to_string(),
            value: Some("open".to_string()),
            ts: "2025-01-01T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"event\""));
        assert!(json.contains("\"name\":\"door\""));
    }

    #[test]
    fn test_cmd_deserialization() {
        let json = r#"{"type":"cmd","name":"arm","exit_delay_s":30,"id":"c1"}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        
        match msg {
            WsMessage::Cmd { name, .. } => {
                assert_eq!(name, "arm");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
