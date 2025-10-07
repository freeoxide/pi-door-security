//! Configuration validation

use super::AppConfig;
use anyhow::{bail, Result};

impl AppConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate client_id
        if self.system.client_id.is_empty() {
            bail!("system.client_id cannot be empty");
        }

        // Validate listen address
        if self.http.listen_addr.is_empty() {
            bail!("http.listen_addr cannot be empty");
        }

        // Validate GPIO pins (must be different)
        let pins = vec![
            ("reed_in", self.gpio.reed_in),
            ("siren_out", self.gpio.siren_out),
            ("floodlight_out", self.gpio.floodlight_out),
            ("radio433_rx_in", self.gpio.radio433_rx_in),
        ];

        for i in 0..pins.len() {
            for j in (i + 1)..pins.len() {
                if pins[i].1 == pins[j].1 {
                    bail!(
                        "GPIO pin conflict: {} and {} both use pin {}",
                        pins[i].0,
                        pins[j].0,
                        pins[i].1
                    );
                }
            }
        }

        // Validate timer values (must be positive)
        if self.timers.exit_delay_s == 0 {
            bail!("timers.exit_delay_s must be greater than 0");
        }
        if self.timers.entry_delay_s == 0 {
            bail!("timers.entry_delay_s must be greater than 0");
        }
        if self.timers.siren_max_s == 0 {
            bail!("timers.siren_max_s must be greater than 0");
        }

        // Validate cloud config if URL is provided
        if let Some(url) = &self.cloud.url {
            if !url.starts_with("wss://") && !url.starts_with("ws://") {
                bail!("cloud.url must start with ws:// or wss://");
            }
        }

        // Validate backoff values
        if self.cloud.backoff_min_s > self.cloud.backoff_max_s {
            bail!(
                "cloud.backoff_min_s ({}) must be <= cloud.backoff_max_s ({})",
                self.cloud.backoff_min_s,
                self.cloud.backoff_max_s
            );
        }

        // Validate queue limits
        if self.cloud.queue_max_events == 0 {
            bail!("cloud.queue_max_events must be greater than 0");
        }
        if self.cloud.queue_max_age_days == 0 {
            bail!("cloud.queue_max_age_days must be greater than 0");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_passes_with_defaults() {
        let config = AppConfig::load().unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_fails_with_duplicate_pins() {
        let mut config = AppConfig::load().unwrap();
        config.gpio.siren_out = config.gpio.reed_in;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_fails_with_invalid_timers() {
        let mut config = AppConfig::load().unwrap();
        config.timers.exit_delay_s = 0;
        assert!(config.validate().is_err());
    }
}
