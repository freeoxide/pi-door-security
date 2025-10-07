//! Systemd watchdog integration

use std::time::Duration;
use tokio::time::interval;

pub struct WatchdogManager {
    #[cfg(feature = "systemd")]
    enabled: bool,
    interval: Duration,
}

impl WatchdogManager {
    pub fn new() -> Self {
        #[cfg(feature = "systemd")]
        {
            // Check if systemd watchdog is enabled
            let watchdog_usec = std::env::var("WATCHDOG_USEC").ok();
            let enabled = watchdog_usec.is_some();
            
            let interval = if let Some(usec) = watchdog_usec {
                let usec: u64 = usec.parse().unwrap_or(15_000_000);
                // Notify at half the watchdog interval
                Duration::from_micros(usec / 2)
            } else {
                Duration::from_secs(15)
            };

            if enabled {
                info!(interval_s = interval.as_secs(), "Systemd watchdog enabled");
            }

            Self { enabled, interval }
        }

        #[cfg(not(feature = "systemd"))]
        {
            Self {
                interval: Duration::from_secs(15),
            }
        }
    }

    /// Start watchdog notification loop
    pub async fn run(&self) {
        #[cfg(feature = "systemd")]
        if !self.enabled {
            return;
        }

        let mut ticker = interval(self.interval);

        loop {
            ticker.tick().await;
            
            #[cfg(feature = "systemd")]
            {
                if let Err(e) = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]) {
                    tracing::error!(error = %e, "Failed to notify systemd watchdog");
                }
                debug!("Sent watchdog keep-alive");
            }
        }
    }

    /// Notify systemd that we're ready
    pub fn notify_ready(&self) {
        #[cfg(feature = "systemd")]
        {
            if self.enabled {
                if let Err(e) = sd_notify::notify(false, &[sd_notify::NotifyState::Ready]) {
                    tracing::error!(error = %e, "Failed to notify systemd ready");
                } else {
                    info!("Notified systemd that service is ready");
                }
            }
        }
    }
}

impl Default for WatchdogManager {
    fn default() -> Self {
        Self::new()
    }
}
