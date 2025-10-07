//! Pi Door Security Client Agent
//! 
//! A security system controller for Raspberry Pi that manages door sensors,
//! alarms, and multiple control interfaces (BLE, RF433, HTTP/WebSocket).

pub mod config;
pub mod events;
pub mod state;
pub mod timers;
pub mod gpio;
pub mod actuators;
pub mod api;
pub mod cloud;
pub mod ble;
pub mod rf433;
pub mod network;
pub mod security;
pub mod observability;
pub mod health;

pub use config::AppConfig;
pub use events::{Event, EventBus};
pub use state::{AlarmState, StateMachine, SharedState};

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Common result type for the application
pub type Result<T> = anyhow::Result<T>;
