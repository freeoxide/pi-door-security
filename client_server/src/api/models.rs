//! API request and response models

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ArmRequest {
    pub exit_delay_s: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisarmRequest {
    pub auto_rearm_s: Option<u64>,
}
