//! Event bus for distributing events across the application

use super::{Event, EventEnvelope};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error};

/// Event bus for distributing events
#[derive(Clone)]
pub struct EventBus {
    /// Sender for new events
    tx: mpsc::UnboundedSender<Event>,
    /// Broadcast channel for subscribers
    broadcast_tx: broadcast::Sender<EventEnvelope>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Event>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (broadcast_tx, _) = broadcast::channel(100);
        
        let bus = Self { tx, broadcast_tx };
        
        (bus, rx)
    }

    /// Emit an event to the bus
    pub fn emit(&self, event: Event) -> anyhow::Result<()> {
        debug!(?event, "Emitting event to bus");
        self.tx.send(event).map_err(|e| {
            error!("Failed to send event to bus: {}", e);
            anyhow::anyhow!("Event bus send failed: {}", e)
        })
    }

    /// Subscribe to all events
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.broadcast_tx.subscribe()
    }

    /// Broadcast an event envelope to all subscribers
    pub fn broadcast(&self, envelope: EventEnvelope) -> anyhow::Result<()> {
        let subscriber_count = self.broadcast_tx.receiver_count();
        debug!(
            event_id = %envelope.id,
            subscribers = subscriber_count,
            "Broadcasting event envelope"
        );
        
        // Ignore send error if there are no subscribers
        if subscriber_count > 0 {
            let _ = self.broadcast_tx.send(envelope);
        }
        
        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;

    #[tokio::test]
    async fn test_event_bus_emit() {
        let (bus, mut rx) = EventBus::new();
        
        let event = Event::DoorOpen;
        bus.emit(event.clone()).unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            Event::DoorOpen => {},
            _ => panic!("Wrong event received"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_subscribe() {
        let (bus, _rx) = EventBus::new();
        let mut sub = bus.subscribe();
        
        let envelope = EventEnvelope::new(
            Event::DoorClose,
            "test".to_string()
        );
        
        bus.broadcast(envelope.clone()).unwrap();
        
        let received = sub.recv().await.unwrap();
        assert_eq!(received.id, envelope.id);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let (bus, _rx) = EventBus::new();
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();
        
        let envelope = EventEnvelope::new(
            Event::UserArm {
                source: EventSource::Local,
                exit_delay_s: Some(30),
            },
            "test".to_string()
        );
        
        bus.broadcast(envelope.clone()).unwrap();
        
        let received1 = sub1.recv().await.unwrap();
        let received2 = sub2.recv().await.unwrap();
        
        assert_eq!(received1.id, envelope.id);
        assert_eq!(received2.id, envelope.id);
    }
}
