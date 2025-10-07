//! Queue manager for offline event handling

use crate::events::{EventEnvelope, EventQueue};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

pub struct QueueManager {
    queue: Arc<Mutex<EventQueue>>,
    batch_size: usize,
}

impl QueueManager {
    pub fn new(queue: EventQueue, batch_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(queue)),
            batch_size,
        }
    }

    /// Enqueue an event for later transmission
    pub async fn enqueue(&self, envelope: EventEnvelope) -> Result<()> {
        let queue = self.queue.lock().await;
        queue.enqueue(envelope)?;
        Ok(())
    }

    /// Replay queued events (call when connection is established)
    pub async fn replay<F>(&self, mut send_fn: F) -> Result<usize>
    where
        F: FnMut(&EventEnvelope) -> Result<()>,
    {
        let mut total_sent = 0;
        
        loop {
            let batch = {
                let queue = self.queue.lock().await;
                queue.dequeue_batch(self.batch_size)?
            };

            if batch.is_empty() {
                break;
            }

            debug!(count = batch.len(), "Replaying event batch");

            let mut sent = Vec::new();
            for envelope in &batch {
                match send_fn(envelope) {
                    Ok(_) => {
                        sent.push(envelope.clone());
                        total_sent += 1;
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to send queued event, stopping replay");
                        break;
                    }
                }
            }

            // Remove successfully sent events
            if !sent.is_empty() {
                let queue = self.queue.lock().await;
                queue.remove(&sent)?;
            }

            // Small delay between batches to avoid overwhelming server
            sleep(Duration::from_millis(100)).await;
        }

        if total_sent > 0 {
            info!(count = total_sent, "Replayed queued events");
        }

        Ok(total_sent)
    }

    /// Get current queue size
    pub async fn size(&self) -> Result<usize> {
        let queue = self.queue.lock().await;
        queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_queue_manager_enqueue() {
        let temp_dir = TempDir::new().unwrap();
        let queue = EventQueue::new(temp_dir.path(), 100, 7).unwrap();
        let mgr = QueueManager::new(queue, 10);

        let envelope = EventEnvelope::new(Event::DoorOpen, "test".to_string());
        mgr.enqueue(envelope).await.unwrap();

        assert_eq!(mgr.size().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_queue_manager_replay() {
        let temp_dir = TempDir::new().unwrap();
        let queue = EventQueue::new(temp_dir.path(), 100, 7).unwrap();
        let mgr = QueueManager::new(queue, 10);

        // Enqueue some events
        for _ in 0..5 {
            let envelope = EventEnvelope::new(Event::DoorOpen, "test".to_string());
            mgr.enqueue(envelope).await.unwrap();
        }

        assert_eq!(mgr.size().await.unwrap(), 5);

        // Replay
        let mut sent_count = 0;
        let count = mgr.replay(|_| {
            sent_count += 1;
            Ok(())
        }).await.unwrap();

        assert_eq!(count, 5);
        assert_eq!(sent_count, 5);
        assert_eq!(mgr.size().await.unwrap(), 0);
    }
}
