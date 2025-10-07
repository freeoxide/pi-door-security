//! Reconnection manager with exponential backoff

use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

pub struct ReconnectManager {
    min_backoff: Duration,
    max_backoff: Duration,
    current_backoff: Duration,
    stable_connection_threshold: Duration,
}

impl ReconnectManager {
    pub fn new(min_backoff_s: u64, max_backoff_s: u64) -> Self {
        let min = Duration::from_secs(min_backoff_s);
        Self {
            min_backoff: min,
            max_backoff: Duration::from_secs(max_backoff_s),
            current_backoff: min,
            stable_connection_threshold: Duration::from_secs(60),
        }
    }

    /// Wait for the current backoff duration, then increase for next time
    pub async fn backoff(&mut self) {
        info!(
            backoff_s = self.current_backoff.as_secs(),
            "Backing off before reconnect"
        );
        sleep(self.current_backoff).await;
        
        // Double the backoff with full jitter
        let next = self.current_backoff * 2;
        self.current_backoff = next.min(self.max_backoff);
        
        // Add jitter (0-50% of backoff)
        let jitter = self.current_backoff / 4;
        let jitter_amount = (rand::random::<f64>() * jitter.as_secs_f64()) as u64;
        self.current_backoff += Duration::from_secs(jitter_amount);
        
        debug!(next_backoff_s = self.current_backoff.as_secs(), "Next backoff calculated");
    }

    /// Reset backoff after a stable connection
    pub fn reset(&mut self) {
        info!("Resetting backoff after stable connection");
        self.current_backoff = self.min_backoff;
    }

    /// Get current backoff duration
    pub fn current(&self) -> Duration {
        self.current_backoff
    }
}

impl Default for ReconnectManager {
    fn default() -> Self {
        Self::new(1, 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases() {
        let mut mgr = ReconnectManager::new(1, 60);
        
        assert_eq!(mgr.current().as_secs(), 1);
        
        // Backoff should roughly double (with jitter)
        tokio_test::block_on(mgr.backoff());
        assert!(mgr.current().as_secs() >= 2 && mgr.current().as_secs() <= 3);
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let mut mgr = ReconnectManager::new(30, 60);
        
        // Should cap at max
        tokio_test::block_on(mgr.backoff());
        tokio_test::block_on(mgr.backoff());
        assert!(mgr.current().as_secs() <= 60);
    }

    #[test]
    fn test_reset() {
        let mut mgr = ReconnectManager::new(1, 60);
        
        tokio_test::block_on(mgr.backoff());
        assert!(mgr.current().as_secs() > 1);
        
        mgr.reset();
        assert_eq!(mgr.current().as_secs(), 1);
    }
}
