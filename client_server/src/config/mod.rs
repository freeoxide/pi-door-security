//! Configuration management module

mod schema;
mod validation;

pub use schema::*;

use anyhow::Result;

/// Load application configuration from various sources
pub fn load_config() -> Result<AppConfig> {
    let config = AppConfig::load()?;
    config.validate()?;
    Ok(config)
}
