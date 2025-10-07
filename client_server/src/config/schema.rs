//! Configuration data structures

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub system: SystemConfig,
    pub network: NetworkConfig,
    pub http: HttpConfig,
    pub ws_local: WsLocalConfig,
    pub cloud: CloudConfig,
    pub gpio: GpioConfig,
    pub timers: TimerConfig,
    pub ble: BleConfig,
    pub rf433: Rf433Config,
}

impl AppConfig {
    /// Load configuration from file and environment
    pub fn load() -> anyhow::Result<Self> {
        let config_path = std::env::var("PI_CLIENT_CONFIG")
            .unwrap_or_else(|_| "/etc/pi-door-client/config.toml".to_string());

        let settings = config::Config::builder()
            // Start with defaults
            .set_default("system.client_id", "pi001")?
            .set_default("system.data_dir", "/var/lib/pi-door-client")?
            .set_default("system.log_level", "info")?
            .set_default("network.prefer", vec!["eth0", "wlan0"])?
            .set_default("network.enable_lte", false)?
            .set_default("http.listen_addr", "0.0.0.0:8080")?
            .set_default("ws_local.enabled", true)?
            .set_default("cloud.heartbeat_s", 20)?
            .set_default("cloud.backoff_min_s", 1)?
            .set_default("cloud.backoff_max_s", 60)?
            .set_default("cloud.queue_max_events", 10000)?
            .set_default("cloud.queue_max_age_days", 7)?
            .set_default("gpio.reed_in", 17)?
            .set_default("gpio.reed_active_low", true)?
            .set_default("gpio.siren_out", 27)?
            .set_default("gpio.floodlight_out", 22)?
            .set_default("gpio.radio433_rx_in", 23)?
            .set_default("gpio.debounce_ms", 50)?
            .set_default("timers.exit_delay_s", 30)?
            .set_default("timers.entry_delay_s", 30)?
            .set_default("timers.auto_rearm_s", 120)?
            .set_default("timers.siren_max_s", 120)?
            .set_default("ble.enabled", true)?
            .set_default("ble.pairing_window_s", 120)?
            .set_default("rf433.enabled", true)?
            .set_default("rf433.allow_disarm", false)?
            .set_default("rf433.debounce_ms", 500)?
            // Try to load from file (may not exist)
            .add_source(
                config::File::with_name(&config_path)
                    .required(false)
            )
            // Override with environment variables
            .add_source(
                config::Environment::with_prefix("PI_CLIENT")
                    .separator("__")
            )
            .build()?;

        let config: AppConfig = settings.try_deserialize()?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub client_id: String,
    pub data_dir: PathBuf,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub prefer: Vec<String>,
    #[serde(default)]
    pub enable_lte: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub listen_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsLocalConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub spki_pins: Vec<String>,
    pub heartbeat_s: u64,
    pub backoff_min_s: u64,
    pub backoff_max_s: u64,
    pub queue_max_events: usize,
    pub queue_max_age_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioConfig {
    pub reed_in: u8,
    pub reed_active_low: bool,
    pub siren_out: u8,
    pub floodlight_out: u8,
    pub radio433_rx_in: u8,
    pub debounce_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfig {
    pub exit_delay_s: u64,
    pub entry_delay_s: u64,
    pub auto_rearm_s: u64,
    pub siren_max_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleConfig {
    pub enabled: bool,
    pub pairing_window_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rf433Config {
    pub enabled: bool,
    pub allow_disarm: bool,
    pub debounce_ms: u64,
    #[serde(default)]
    pub mappings: Vec<Rf433Mapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rf433Mapping {
    pub code: String,
    pub action: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            prefer: vec!["eth0".to_string(), "wlan0".to_string()],
            enable_lte: false,
        }
    }
}
