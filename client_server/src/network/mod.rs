//! Network redundancy manager for interface selection and failover

use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn};

/// Network interface information
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub priority: usize,
    pub is_up: bool,
    pub has_carrier: bool,
}

/// Network connectivity status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectivityStatus {
    Online,
    Offline,
}

/// Network redundancy manager
pub struct NetworkManager {
    preferred_interfaces: Vec<String>,
    current_interface: Option<String>,
    connectivity_status: ConnectivityStatus,
}

impl NetworkManager {
    /// Create a new network manager with interface priority
    pub fn new(preferred_interfaces: Vec<String>) -> Self {
        info!(
            interfaces = ?preferred_interfaces,
            "Initializing network manager"
        );
        
        Self {
            preferred_interfaces,
            current_interface: None,
            connectivity_status: ConnectivityStatus::Offline,
        }
    }

    /// Start monitoring network interfaces
    pub async fn start_monitoring(&mut self) {
        let mut check_interval = interval(Duration::from_secs(5));
        
        loop {
            check_interval.tick().await;
            self.check_and_update_interface().await;
        }
    }

    /// Check interfaces and select the best available one
    async fn check_and_update_interface(&mut self) {
        let available_interfaces = self.get_available_interfaces().await;
        
        // Find the highest priority available interface
        let best_interface = self.select_best_interface(&available_interfaces);
        
        // Update current interface if changed
        if best_interface != self.current_interface {
            match &best_interface {
                Some(iface) => {
                    info!(
                        old = ?self.current_interface,
                        new = iface,
                        "Network interface changed"
                    );
                    self.current_interface = Some(iface.clone());
                    self.connectivity_status = ConnectivityStatus::Online;
                }
                None => {
                    warn!("No network interfaces available");
                    self.current_interface = None;
                    self.connectivity_status = ConnectivityStatus::Offline;
                }
            }
        }
    }

    /// Get list of available interfaces
    async fn get_available_interfaces(&self) -> Vec<NetworkInterface> {
        let mut interfaces = Vec::new();
        
        for (priority, name) in self.preferred_interfaces.iter().enumerate() {
            let interface = self.check_interface_status(name).await;
            if interface.is_up && interface.has_carrier {
                interfaces.push(NetworkInterface {
                    name: name.clone(),
                    priority,
                    is_up: interface.is_up,
                    has_carrier: interface.has_carrier,
                });
            }
        }
        
        debug!(available = interfaces.len(), "Available network interfaces");
        interfaces
    }

    /// Check the status of a specific interface
    async fn check_interface_status(&self, name: &str) -> NetworkInterface {
        // Read interface status from /sys/class/net/
        let operstate_path = format!("/sys/class/net/{}/operstate", name);
        let carrier_path = format!("/sys/class/net/{}/carrier", name);
        
        let is_up = tokio::fs::read_to_string(&operstate_path)
            .await
            .map(|s| s.trim() == "up")
            .unwrap_or(false);
        
        let has_carrier = tokio::fs::read_to_string(&carrier_path)
            .await
            .map(|s| s.trim() == "1")
            .unwrap_or(false);
        
        if is_up && has_carrier {
            debug!(interface = name, "Interface available");
        } else {
            debug!(
                interface = name,
                is_up,
                has_carrier,
                "Interface unavailable"
            );
        }
        
        NetworkInterface {
            name: name.to_string(),
            priority: 0,
            is_up,
            has_carrier,
        }
    }

    /// Select the best interface based on priority
    fn select_best_interface(&self, interfaces: &[NetworkInterface]) -> Option<String> {
        interfaces
            .iter()
            .min_by_key(|i| i.priority) // Lower priority number = higher priority
            .map(|i| i.name.clone())
    }

    /// Get current active interface
    pub fn current_interface(&self) -> Option<&str> {
        self.current_interface.as_deref()
    }

    /// Get current connectivity status
    pub fn connectivity_status(&self) -> ConnectivityStatus {
        self.connectivity_status
    }

    /// Test internet connectivity via heartbeat
    pub async fn test_connectivity(&self) -> bool {
        // In production, this would ping a reliable endpoint or check DNS
        // For now, assume online if we have an interface
        self.current_interface.is_some()
    }

    /// Wait for connectivity to be restored
    pub async fn wait_for_connectivity(&mut self, timeout: Duration) -> bool {
        let start = tokio::time::Instant::now();
        
        while start.elapsed() < timeout {
            if self.test_connectivity().await {
                return true;
            }
            sleep(Duration::from_secs(1)).await;
        }
        
        false
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new(vec!["eth0".to_string(), "wlan0".to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let manager = NetworkManager::new(vec!["eth0".to_string(), "wlan0".to_string()]);
        assert_eq!(manager.preferred_interfaces.len(), 2);
        assert_eq!(manager.connectivity_status, ConnectivityStatus::Offline);
    }

    #[tokio::test]
    async fn test_interface_selection() {
        let manager = NetworkManager::new(vec!["eth0".to_string(), "wlan0".to_string()]);
        
        let interfaces = vec![
            NetworkInterface {
                name: "wlan0".to_string(),
                priority: 1,
                is_up: true,
                has_carrier: true,
            },
            NetworkInterface {
                name: "eth0".to_string(),
                priority: 0,
                is_up: true,
                has_carrier: true,
            },
        ];
        
        let best = manager.select_best_interface(&interfaces);
        assert_eq!(best, Some("eth0".to_string())); // Lower priority wins
    }

    #[tokio::test]
    async fn test_connectivity_status() {
        let mut manager = NetworkManager::new(vec!["eth0".to_string()]);
        assert_eq!(manager.connectivity_status(), ConnectivityStatus::Offline);
        
        // Simulate interface becoming available
        manager.current_interface = Some("eth0".to_string());
        manager.connectivity_status = ConnectivityStatus::Online;
        
        assert_eq!(manager.connectivity_status(), ConnectivityStatus::Online);
        assert_eq!(manager.current_interface(), Some("eth0"));
    }

    #[tokio::test]
    async fn test_connectivity_check() {
        let mut manager = NetworkManager::new(vec!["eth0".to_string()]);
        
        // No interface = offline
        assert!(!manager.test_connectivity().await);
        
        // With interface = online
        manager.current_interface = Some("eth0".to_string());
        assert!(manager.test_connectivity().await);
    }
}
