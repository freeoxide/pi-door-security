//! Mock GPIO implementation for testing and development

use super::traits::{Edge, GpioController};
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{debug, info};

/// Mock GPIO controller for testing
#[derive(Clone)]
pub struct MockGpio {
    state: Arc<RwLock<MockGpioState>>,
    door_edge_notify: Arc<Notify>,
}

#[derive(Debug)]
struct MockGpioState {
    door_open: bool,
    siren: bool,
    floodlight: bool,
    initialized: bool,
}

impl Default for MockGpioState {
    fn default() -> Self {
        Self {
            door_open: false,
            siren: false,
            floodlight: false,
            initialized: false,
        }
    }
}

impl MockGpio {
    /// Create a new mock GPIO controller
    pub fn new() -> Self {
        info!("Creating mock GPIO controller");
        Self {
            state: Arc::new(RwLock::new(MockGpioState::default())),
            door_edge_notify: Arc::new(Notify::new()),
        }
    }

    /// Simulate door opening (for testing)
    pub fn simulate_door_open(&self) {
        debug!("Simulating door open");
        {
            let mut state = self.state.write();
            state.door_open = true;
        }
        self.door_edge_notify.notify_waiters();
    }

    /// Simulate door closing (for testing)
    pub fn simulate_door_close(&self) {
        debug!("Simulating door close");
        {
            let mut state = self.state.write();
            state.door_open = false;
        }
        self.door_edge_notify.notify_waiters();
    }

    /// Get current mock state (for testing)
    pub fn get_state(&self) -> (bool, bool, bool) {
        let state = self.state.read();
        (state.door_open, state.siren, state.floodlight)
    }
}

impl Default for MockGpio {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GpioController for MockGpio {
    async fn initialize(&mut self) -> Result<()> {
        info!("Initializing mock GPIO");
        let mut state = self.state.write();
        
        // Set to safe state
        state.siren = false;
        state.floodlight = false;
        state.door_open = false;
        state.initialized = true;
        
        debug!("Mock GPIO initialized successfully");
        Ok(())
    }

    async fn read_door_sensor(&self) -> Result<bool> {
        let state = self.state.read();
        Ok(state.door_open)
    }

    async fn set_siren(&self, on: bool) -> Result<()> {
        debug!(on, "Setting mock siren");
        let mut state = self.state.write();
        state.siren = on;
        Ok(())
    }

    async fn set_floodlight(&self, on: bool) -> Result<()> {
        debug!(on, "Setting mock floodlight");
        let mut state = self.state.write();
        state.floodlight = on;
        Ok(())
    }

    async fn wait_for_door_edge(&self) -> Result<Edge> {
        // Wait for notification
        self.door_edge_notify.notified().await;
        
        // Determine edge direction
        let door_open = self.read_door_sensor().await?;
        let edge = if door_open { Edge::Rising } else { Edge::Falling };
        
        debug!(?edge, "Door edge detected");
        Ok(edge)
    }

    fn emergency_shutdown(&self) {
        info!("Emergency shutdown - setting mock outputs to safe state");
        let mut state = self.state.write();
        state.siren = false;
        state.floodlight = false;
    }

    async fn get_siren_state(&self) -> Result<bool> {
        let state = self.state.read();
        Ok(state.siren)
    }

    async fn get_floodlight_state(&self) -> Result<bool> {
        let state = self.state.read();
        Ok(state.floodlight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_gpio_initialization() {
        let mut gpio = MockGpio::new();
        gpio.initialize().await.unwrap();

        let (door, siren, flood) = gpio.get_state();
        assert!(!door);
        assert!(!siren);
        assert!(!flood);
    }

    #[tokio::test]
    async fn test_mock_gpio_siren_control() {
        let mut gpio = MockGpio::new();
        gpio.initialize().await.unwrap();

        gpio.set_siren(true).await.unwrap();
        assert!(gpio.get_siren_state().await.unwrap());

        gpio.set_siren(false).await.unwrap();
        assert!(!gpio.get_siren_state().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_gpio_door_simulation() {
        let mut gpio = MockGpio::new();
        gpio.initialize().await.unwrap();

        gpio.simulate_door_open();
        assert!(gpio.read_door_sensor().await.unwrap());

        gpio.simulate_door_close();
        assert!(!gpio.read_door_sensor().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_gpio_edge_detection() {
        let mut gpio = MockGpio::new();
        gpio.initialize().await.unwrap();

        // Spawn task to wait for edge
        let gpio_clone = gpio.clone();
        let handle = tokio::spawn(async move {
            gpio_clone.wait_for_door_edge().await
        });

        // Give the task time to start waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Trigger door open
        gpio.simulate_door_open();

        // Should receive edge event
        let edge = handle.await.unwrap().unwrap();
        assert_eq!(edge, Edge::Rising);
    }

    #[test]
    fn test_emergency_shutdown() {
        let gpio = MockGpio::new();
        
        // Set some outputs
        {
            let mut state = gpio.state.write();
            state.siren = true;
            state.floodlight = true;
        }

        // Emergency shutdown
        gpio.emergency_shutdown();

        // Verify safe state
        let (_, siren, flood) = gpio.get_state();
        assert!(!siren);
        assert!(!flood);
    }
}
