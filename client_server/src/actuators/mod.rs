//! Actuator control module

use crate::gpio::GpioController;
use crate::state::{ActuatorState, AppState};
use anyhow::Result;
use std::sync::Arc;
use tracing::debug;

/// Actuator controller manages siren and floodlight outputs
pub struct ActuatorController {
    gpio: Arc<dyn GpioController>,
    state: AppState,
}

impl ActuatorController {
    pub fn new(gpio: Arc<dyn GpioController>, state: AppState) -> Self {
        Self { gpio, state }
    }

    /// Update actuators based on current state
    pub async fn update(&self) -> Result<()> {
        let target_state = {
            let state = self.state.read();
            state.actuators
        };

        self.apply_state(target_state).await
    }

    /// Apply actuator state to GPIO
    async fn apply_state(&self, target: ActuatorState) -> Result<()> {
        debug!(?target, "Applying actuator state");

        self.gpio.set_siren(target.siren).await?;
        self.gpio.set_floodlight(target.floodlight).await?;

        Ok(())
    }
}
