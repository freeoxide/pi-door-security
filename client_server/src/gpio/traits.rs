//! GPIO controller trait definition

use anyhow::Result;
use async_trait::async_trait;

/// GPIO edge detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Rising,
    Falling,
    Both,
}

/// GPIO controller trait for hardware abstraction
#[async_trait]
pub trait GpioController: Send + Sync {
    /// Initialize GPIO pins
    async fn initialize(&mut self) -> Result<()>;

    /// Read the door sensor state (true = open, false = closed)
    async fn read_door_sensor(&self) -> Result<bool>;

    /// Set siren relay state
    async fn set_siren(&self, on: bool) -> Result<()>;

    /// Set floodlight relay state
    async fn set_floodlight(&self, on: bool) -> Result<()>;

    /// Wait for a door sensor edge event
    async fn wait_for_door_edge(&self) -> Result<Edge>;

    /// Emergency shutdown - set all outputs to safe state
    /// This should be synchronous for panic handlers
    fn emergency_shutdown(&self);

    /// Get current siren state
    async fn get_siren_state(&self) -> Result<bool>;

    /// Get current floodlight state
    async fn get_floodlight_state(&self) -> Result<bool>;
}
