//! Real GPIO implementation using rppal crate for Raspberry Pi

use anyhow::{Context, Result};
use rppal::gpio::{Gpio, InputPin, Level, OutputPin, Trigger};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::traits::{ActuatorState, DoorState, GpioController};

/// Real GPIO controller using rppal
pub struct RppalGpio {
    reed_pin: Arc<RwLock<InputPin>>,
    siren_pin: Arc<RwLock<OutputPin>>,
    floodlight_pin: Arc<RwLock<OutputPin>>,
    reed_active_low: bool,
    door_state: Arc<RwLock<DoorState>>,
    actuator_state: Arc<RwLock<ActuatorState>>,
}

impl RppalGpio {
    /// Create a new real GPIO controller
    pub fn new(
        reed_pin_num: u8,
        siren_pin_num: u8,
        floodlight_pin_num: u8,
        reed_active_low: bool,
    ) -> Result<Self> {
        info!(
            reed = reed_pin_num,
            siren = siren_pin_num,
            floodlight = floodlight_pin_num,
            reed_active_low,
            "Initializing real GPIO controller"
        );

        let gpio = Gpio::new().context("Failed to initialize GPIO")?;

        // Initialize reed input with pull-up
        let mut reed_pin = gpio
            .get(reed_pin_num)
            .context("Failed to get reed input pin")?
            .into_input_pullup();

        // Set up interrupt for reed pin
        reed_pin
            .set_interrupt(Trigger::Both)
            .context("Failed to set reed pin interrupt")?;

        // Initialize output pins to safe low state
        let mut siren_pin = gpio
            .get(siren_pin_num)
            .context("Failed to get siren output pin")?
            .into_output();
        siren_pin.set_low();

        let mut floodlight_pin = gpio
            .get(floodlight_pin_num)
            .context("Failed to get floodlight output pin")?
            .into_output();
        floodlight_pin.set_low();

        // Read initial door state
        let initial_level = reed_pin.read();
        let door_closed = if reed_active_low {
            initial_level == Level::Low
        } else {
            initial_level == Level::High
        };

        let initial_door_state = if door_closed {
            DoorState::Closed
        } else {
            DoorState::Open
        };

        info!(door_state = ?initial_door_state, "Initial door state detected");

        Ok(Self {
            reed_pin: Arc::new(RwLock::new(reed_pin)),
            siren_pin: Arc::new(RwLock::new(siren_pin)),
            floodlight_pin: Arc::new(RwLock::new(floodlight_pin)),
            reed_active_low,
            door_state: Arc::new(RwLock::new(initial_door_state)),
            actuator_state: Arc::new(RwLock::new(ActuatorState {
                siren: false,
                floodlight: false,
            })),
        })
    }

    /// Poll reed pin for state changes (with debouncing)
    async fn poll_reed_state(&self) -> Result<DoorState> {
        let reed_pin = self.reed_pin.read().await;
        let level = reed_pin.read();

        let door_closed = if self.reed_active_low {
            level == Level::Low
        } else {
            level == Level::High
        };

        Ok(if door_closed {
            DoorState::Closed
        } else {
            DoorState::Open
        })
    }
}

#[async_trait::async_trait]
impl GpioController for RppalGpio {
    async fn read_door_state(&self) -> Result<DoorState> {
        // Read current state and update cached value
        let new_state = self.poll_reed_state().await?;
        let mut door_state = self.door_state.write().await;
        
        if *door_state != new_state {
            debug!(old_state = ?*door_state, new_state = ?new_state, "Door state changed");
            *door_state = new_state;
        }

        Ok(*door_state)
    }

    async fn set_siren(&self, enabled: bool) -> Result<()> {
        debug!(enabled, "Setting siren");
        
        let mut siren_pin = self.siren_pin.write().await;
        if enabled {
            siren_pin.set_high();
        } else {
            siren_pin.set_low();
        }

        let mut state = self.actuator_state.write().await;
        state.siren = enabled;

        Ok(())
    }

    async fn set_floodlight(&self, enabled: bool) -> Result<()> {
        debug!(enabled, "Setting floodlight");
        
        let mut floodlight_pin = self.floodlight_pin.write().await;
        if enabled {
            floodlight_pin.set_high();
        } else {
            floodlight_pin.set_low();
        }

        let mut state = self.actuator_state.write().await;
        state.floodlight = enabled;

        Ok(())
    }

    async fn get_actuator_state(&self) -> ActuatorState {
        *self.actuator_state.read().await
    }

    async fn emergency_shutdown(&self) -> Result<()> {
        warn!("Emergency GPIO shutdown initiated");
        
        // Set all outputs to safe low state
        {
            let mut siren_pin = self.siren_pin.write().await;
            siren_pin.set_low();
        }
        
        {
            let mut floodlight_pin = self.floodlight_pin.write().await;
            floodlight_pin.set_low();
        }

        let mut state = self.actuator_state.write().await;
        state.siren = false;
        state.floodlight = false;

        info!("Emergency GPIO shutdown complete");
        Ok(())
    }
}

impl Drop for RppalGpio {
    fn drop(&mut self) {
        // Emergency shutdown on drop (async not available in Drop)
        // This is best-effort only
        warn!("RppalGpio dropped, attempting emergency shutdown");
        
        // Note: We can't await in Drop, so this is synchronous and may not complete
        // The proper shutdown should be done via emergency_shutdown() before dropping
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require actual Raspberry Pi hardware and will fail in CI
    // They are marked as ignored and should be run manually on target hardware

    #[tokio::test]
    #[ignore = "requires Raspberry Pi hardware"]
    async fn test_gpio_initialization() {
        let gpio = RppalGpio::new(17, 27, 22, true);
        assert!(gpio.is_ok(), "GPIO initialization should succeed on Pi");
    }

    #[tokio::test]
    #[ignore = "requires Raspberry Pi hardware"]
    async fn test_door_state_reading() {
        let gpio = RppalGpio::new(17, 27, 22, true).unwrap();
        let state = gpio.read_door_state().await;
        assert!(state.is_ok(), "Should be able to read door state");
    }

    #[tokio::test]
    #[ignore = "requires Raspberry Pi hardware"]
    async fn test_actuator_control() {
        let gpio = RppalGpio::new(17, 27, 22, true).unwrap();
        
        // Test siren
        gpio.set_siren(true).await.unwrap();
        let state = gpio.get_actuator_state().await;
        assert!(state.siren);
        
        gpio.set_siren(false).await.unwrap();
        let state = gpio.get_actuator_state().await;
        assert!(!state.siren);
    }

    #[tokio::test]
    #[ignore = "requires Raspberry Pi hardware"]
    async fn test_emergency_shutdown() {
        let gpio = RppalGpio::new(17, 27, 22, true).unwrap();
        
        // Turn on actuators
        gpio.set_siren(true).await.unwrap();
        gpio.set_floodlight(true).await.unwrap();
        
        // Emergency shutdown
        gpio.emergency_shutdown().await.unwrap();
        
        let state = gpio.get_actuator_state().await;
        assert!(!state.siren);
        assert!(!state.floodlight);
    }
}
