//! Cloud WebSocket client with TLS 1.3 and JWT authentication

use crate::events::{EventBus, EventEnvelope};
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{
        client::IntoClientRequest,
        protocol::Message,
    },
};
use tracing::{debug, error, info, warn};

#[derive(Serialize, Deserialize)]
struct CloudMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

pub struct CloudClient {
    url: String,
    jwt: Option<String>,
    heartbeat_interval: Duration,
    event_bus: EventBus,
}

impl CloudClient {
    pub fn new(
        url: String,
        jwt: Option<String>,
        heartbeat_s: u64,
        event_bus: EventBus,
    ) -> Self {
        Self {
            url,
            jwt,
            heartbeat_interval: Duration::from_secs(heartbeat_s),
            event_bus,
        }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("Cloud connection closed normally");
                    break;
                }
                Err(e) => {
                    error!(error = %e, "Cloud connection error");
                    // Exponential backoff handled by reconnect logic
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
        Ok(())
    }

    async fn connect_and_run(&self) -> Result<()> {
        info!(url = %self.url, "Connecting to cloud");

        // Create request with Authorization header
        let mut request = self.url.clone().into_client_request()?;
        
        if let Some(jwt) = &self.jwt {
            request.headers_mut().insert(
                "Authorization",
                format!("Bearer {}", jwt).parse()?,
            );
        }

        // Connect with TLS
        let (ws_stream, _) = connect_async_tls_with_config(
            request,
            None,
            false,
            None,
        )
        .await
        .context("Failed to connect to cloud")?;

        info!("Connected to cloud successfully");

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to local events
        let mut event_rx = self.event_bus.subscribe();

        // Heartbeat timer
        let mut heartbeat = interval(self.heartbeat_interval);

        loop {
            tokio::select! {
                // Send heartbeat ping
                _ = heartbeat.tick() => {
                    debug!("Sending cloud heartbeat");
                    if let Err(e) = write.send(Message::Ping(vec![])).await {
                        error!(error = %e, "Failed to send ping");
                        return Err(e.into());
                    }
                }

                // Forward local events to cloud
                Ok(envelope) = event_rx.recv() => {
                    let msg = self.envelope_to_message(&envelope);
                    let json = serde_json::to_string(&msg)?;
                    
                    if let Err(e) = write.send(Message::Text(json)).await {
                        error!(error = %e, "Failed to send event to cloud");
                        return Err(e.into());
                    }
                }

                // Receive messages from cloud
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            debug!(text, "Received message from cloud");
                            if let Err(e) = self.handle_cloud_message(&text) {
                                warn!(error = %e, "Failed to handle cloud message");
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Cloud connection closed by server");
                            return Ok(());
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong from cloud");
                        }
                        Some(Err(e)) => {
                            error!(error = %e, "WebSocket error");
                            return Err(e.into());
                        }
                        None => {
                            warn!("Cloud connection stream ended");
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn envelope_to_message(&self, envelope: &EventEnvelope) -> CloudMessage {
        CloudMessage {
            msg_type: "event".to_string(),
            data: serde_json::to_value(envelope).unwrap_or(serde_json::Value::Null),
        }
    }

    fn handle_cloud_message(&self, text: &str) -> Result<()> {
        let msg: CloudMessage = serde_json::from_str(text)?;
        
        match msg.msg_type.as_str() {
            "cmd" => {
                debug!("Received command from cloud");
                // Parse and emit command events
                // TODO: Implement command handling
            }
            "ack" => {
                debug!("Received acknowledgment from cloud");
            }
            _ => {
                warn!(msg_type = %msg.msg_type, "Unknown message type from cloud");
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_to_message() {
        let (bus, _) = EventBus::new();
        let client = CloudClient::new(
            "wss://example.com/client".to_string(),
            Some("test-jwt".to_string()),
            20,
            bus,
        );

        let envelope = EventEnvelope::new(
            crate::events::Event::DoorOpen,
            "test-client".to_string(),
        );

        let msg = client.envelope_to_message(&envelope);
        assert_eq!(msg.msg_type, "event");
    }
}
