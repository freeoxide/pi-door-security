//! Health monitoring and systemd watchdog integration

mod watchdog;

pub use watchdog::WatchdogManager;

pub struct HealthMonitor {
    watchdog: WatchdogManager,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            watchdog: WatchdogManager::new(),
        }
    }

    pub fn watchdog(&self) -> &WatchdogManager {
        &self.watchdog
    }
}
