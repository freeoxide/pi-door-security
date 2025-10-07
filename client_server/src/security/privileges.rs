//! Privilege dropping after socket binding

use anyhow::{Context, Result};
use tracing::info;

/// Drop privileges to specified user
#[cfg(unix)]
pub fn drop_privileges(username: &str) -> Result<()> {
    use nix::unistd::{setgid, setuid, Gid, Uid, User};

    let user = User::from_name(username)
        .context("Failed to lookup user")?
        .ok_or_else(|| anyhow::anyhow!("User '{}' not found", username))?;

    // Set GID first (must be done before dropping UID)
    setgid(Gid::from_raw(user.gid.as_raw()))
        .context("Failed to set GID")?;

    // Then set UID
    setuid(Uid::from_raw(user.uid.as_raw()))
        .context("Failed to set UID")?;

    info!(user = username, uid = user.uid.as_raw(), gid = user.gid.as_raw(), 
          "Dropped privileges");

    Ok(())
}

#[cfg(not(unix))]
pub fn drop_privileges(_username: &str) -> Result<()> {
    warn!("Privilege dropping not supported on non-Unix systems");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn test_drop_privileges_requires_valid_user() {
        // This test will fail if run as non-root, which is expected
        let result = drop_privileges("nonexistent_user_12345");
        assert!(result.is_err());
    }
}
