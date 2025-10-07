//! Disk-backed event queue for offline persistence

use super::EventEnvelope;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use std::path::Path;
use tracing::{debug, warn};

/// Event queue with disk persistence
pub struct EventQueue {
    db: sled::Db,
    max_events: usize,
    max_age: Duration,
}

impl EventQueue {
    /// Create or open an event queue at the specified path
    pub fn new<P: AsRef<Path>>(
        path: P,
        max_events: usize,
        max_age_days: u32,
    ) -> Result<Self> {
        let db = sled::open(path.as_ref())
            .context("Failed to open event queue database")?;

        let max_age = Duration::days(max_age_days as i64);

        Ok(Self {
            db,
            max_events,
            max_age,
        })
    }

    /// Enqueue an event envelope
    pub fn enqueue(&self, envelope: EventEnvelope) -> Result<()> {
        let key = self.make_key(&envelope.timestamp, &envelope.id);
        let value = serde_json::to_vec(&envelope)
            .context("Failed to serialize event envelope")?;

        self.db.insert(key, value)
            .context("Failed to insert event into queue")?;

        debug!(
            event_id = %envelope.id,
            queue_size = self.len()?,
            "Event enqueued"
        );

        // Prune if necessary
        self.prune()?;

        Ok(())
    }

    /// Dequeue a batch of events (oldest first)
    pub fn dequeue_batch(&self, limit: usize) -> Result<Vec<EventEnvelope>> {
        let mut events = Vec::new();

        for result in self.db.iter().take(limit) {
            let (_key, value) = result.context("Failed to read from queue")?;
            let envelope: EventEnvelope = serde_json::from_slice(&value)
                .context("Failed to deserialize event envelope")?;
            events.push(envelope);
        }

        debug!(count = events.len(), "Dequeued event batch");
        Ok(events)
    }

    /// Remove events from the queue by their IDs
    pub fn remove(&self, envelopes: &[EventEnvelope]) -> Result<()> {
        for envelope in envelopes {
            let key = self.make_key(&envelope.timestamp, &envelope.id);
            self.db.remove(key)
                .context("Failed to remove event from queue")?;
        }

        debug!(count = envelopes.len(), "Removed events from queue");
        Ok(())
    }

    /// Get the current queue size
    pub fn len(&self) -> Result<usize> {
        Ok(self.db.len())
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Clear all events from the queue
    pub fn clear(&self) -> Result<()> {
        self.db.clear().context("Failed to clear queue")?;
        debug!("Queue cleared");
        Ok(())
    }

    /// Prune old events based on max_events and max_age
    fn prune(&self) -> Result<()> {
        let current_len = self.len()?;
        let cutoff_time = Utc::now() - self.max_age;

        // Prune by age
        let mut keys_to_remove = Vec::new();
        for result in self.db.iter() {
            let (key, value) = result.context("Failed to read from queue during pruning")?;
            let envelope: EventEnvelope = serde_json::from_slice(&value)
                .context("Failed to deserialize during pruning")?;

            if envelope.timestamp < cutoff_time {
                keys_to_remove.push(key.to_vec());
            }
        }

        for key in &keys_to_remove {
            self.db.remove(key).context("Failed to remove old event")?;
        }

        if !keys_to_remove.is_empty() {
            warn!(
                removed = keys_to_remove.len(),
                cutoff = %cutoff_time,
                "Pruned old events from queue"
            );
        }

        // Prune by count (keep only max_events newest)
        let after_age_prune = self.len()?;
        if after_age_prune > self.max_events {
            let to_remove = after_age_prune - self.max_events;
            let mut removed = 0;

            for result in self.db.iter().take(to_remove) {
                let (key, _) = result.context("Failed to read during count pruning")?;
                self.db.remove(key).context("Failed to remove excess event")?;
                removed += 1;
            }

            if removed > 0 {
                warn!(
                    removed,
                    max_events = self.max_events,
                    "Pruned excess events from queue"
                );
            }
        }

        Ok(())
    }

    /// Create a sortable key from timestamp and UUID
    fn make_key(&self, timestamp: &DateTime<Utc>, id: &uuid::Uuid) -> Vec<u8> {
        // Use timestamp as primary sort key for chronological ordering
        let ts_nanos = timestamp.timestamp_nanos_opt().unwrap_or(0);
        let mut key = ts_nanos.to_be_bytes().to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;
    use tempfile::TempDir;

    #[test]
    fn test_queue_enqueue_dequeue() {
        let temp_dir = TempDir::new().unwrap();
        let queue = EventQueue::new(temp_dir.path(), 100, 7).unwrap();

        let envelope = EventEnvelope::new(Event::DoorOpen, "test".to_string());
        queue.enqueue(envelope.clone()).unwrap();

        assert_eq!(queue.len().unwrap(), 1);

        let batch = queue.dequeue_batch(10).unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].id, envelope.id);
    }

    #[test]
    fn test_queue_remove() {
        let temp_dir = TempDir::new().unwrap();
        let queue = EventQueue::new(temp_dir.path(), 100, 7).unwrap();

        let envelope = EventEnvelope::new(Event::DoorClose, "test".to_string());
        queue.enqueue(envelope.clone()).unwrap();
        assert_eq!(queue.len().unwrap(), 1);

        queue.remove(&[envelope]).unwrap();
        assert_eq!(queue.len().unwrap(), 0);
    }

    #[test]
    fn test_queue_max_events() {
        let temp_dir = TempDir::new().unwrap();
        let queue = EventQueue::new(temp_dir.path(), 5, 7).unwrap();

        // Add 10 events
        for _ in 0..10 {
            let envelope = EventEnvelope::new(Event::DoorOpen, "test".to_string());
            queue.enqueue(envelope).unwrap();
        }

        // Should have pruned down to 5
        assert_eq!(queue.len().unwrap(), 5);
    }

    #[test]
    fn test_queue_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let envelope = EventEnvelope::new(Event::DoorOpen, "test".to_string());

        // Create queue, add event, drop
        {
            let queue = EventQueue::new(path, 100, 7).unwrap();
            queue.enqueue(envelope.clone()).unwrap();
        }

        // Reopen queue and verify event persisted
        {
            let queue = EventQueue::new(path, 100, 7).unwrap();
            assert_eq!(queue.len().unwrap(), 1);

            let batch = queue.dequeue_batch(10).unwrap();
            assert_eq!(batch[0].id, envelope.id);
        }
    }
}
