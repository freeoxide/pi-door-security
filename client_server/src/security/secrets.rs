//! Secure secret storage and management

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Secure storage for secrets (JWT tokens, API keys, etc.)
pub struct SecretStore {
    secrets_path: PathBuf,
}

impl SecretStore {
    /// Create a new secret store
    pub fn new<P: AsRef<Path>>(secrets_path: P) -> Self {
        Self {
            secrets_path: secrets_path.as_ref().to_path_buf(),
        }
    }

    /// Load JWT token from secret file
    pub fn load_jwt_token(&self) -> Result<Option<String>> {
        self.load_secret("PI_CLIENT_JWT")
    }

    /// Load API key from secret file
    pub fn load_api_key(&self) -> Result<Option<String>> {
        self.load_secret("PI_CLIENT_API_KEY")
    }

    /// Load a specific secret by key name
    pub fn load_secret(&self, key: &str) -> Result<Option<String>> {
        // First check environment variable
        if let Ok(value) = std::env::var(key) {
            debug!(key, "Loaded secret from environment variable");
            return Ok(Some(value));
        }

        // Then check secret file
        if !self.secrets_path.exists() {
            debug!(
                path = ?self.secrets_path,
                "Secret file does not exist"
            );
            return Ok(None);
        }

        // Verify file permissions (Unix only)
        #[cfg(unix)]
        self.verify_secret_file_permissions()?;

        // Read and parse secret file
        let contents = fs::read_to_string(&self.secrets_path)
            .with_context(|| format!("Failed to read secret file: {:?}", self.secrets_path))?;

        for line in contents.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE format
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key {
                    let value = v.trim().trim_matches('"').trim_matches('\'');
                    debug!(key, "Loaded secret from file");
                    return Ok(Some(value.to_string()));
                }
            }
        }

        debug!(key, "Secret not found");
        Ok(None)
    }

    /// Verify secret file has secure permissions (mode 600)
    #[cfg(unix)]
    fn verify_secret_file_permissions(&self) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(&self.secrets_path)
            .context("Failed to get secret file metadata")?;
        
        let permissions = metadata.permissions();
        let mode = permissions.mode() & 0o777;

        // Check if file is world-readable or group-readable
        if mode & 0o077 != 0 {
            warn!(
                path = ?self.secrets_path,
                mode = format!("{:o}", mode),
                "Secret file has insecure permissions (should be 600)"
            );
            
            // Try to fix permissions
            let mut perms = permissions;
            perms.set_mode(0o600);
            fs::set_permissions(&self.secrets_path, perms)
                .context("Failed to set secure permissions on secret file")?;
            
            info!(
                path = ?self.secrets_path,
                "Fixed secret file permissions to 600"
            );
        }

        Ok(())
    }

    /// Save a secret to the secret file
    pub fn save_secret(&self, key: &str, value: &str) -> Result<()> {
        let contents = if self.secrets_path.exists() {
            fs::read_to_string(&self.secrets_path)
                .context("Failed to read existing secret file")?
        } else {
            String::new()
        };

        // Check if key already exists and update it
        let mut found = false;
        let mut new_contents = String::new();
        
        for line in contents.lines() {
            if line.trim().starts_with(&format!("{}=", key)) {
                new_contents.push_str(&format!("{}={}\n", key, value));
                found = true;
            } else {
                new_contents.push_str(line);
                new_contents.push('\n');
            }
        }

        // If not found, append it
        if !found {
            new_contents.push_str(&format!("{}={}\n", key, value));
        }

        // Create parent directory if needed
        if let Some(parent) = self.secrets_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create secrets directory")?;
        }

        // Write with secure permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(&self.secrets_path, new_contents.as_bytes())
                .context("Failed to write secret file")?;
            
            let mut perms = fs::metadata(&self.secrets_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.secrets_path, perms)
                .context("Failed to set permissions")?;
        }

        #[cfg(not(unix))]
        {
            fs::write(&self.secrets_path, new_contents.as_bytes())
                .context("Failed to write secret file")?;
        }

        info!(key, path = ?self.secrets_path, "Secret saved");
        Ok(())
    }

    /// Rotate JWT token (load new one, save old as backup)
    pub fn rotate_jwt_token(&self, new_token: &str) -> Result<()> {
        // Save old token as backup if it exists
        if let Ok(Some(old_token)) = self.load_jwt_token() {
            self.save_secret("PI_CLIENT_JWT_OLD", &old_token)?;
        }

        // Save new token
        self.save_secret("PI_CLIENT_JWT", new_token)?;
        
        info!("JWT token rotated successfully");
        Ok(())
    }

    /// Generate a random API key
    pub fn generate_api_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        const KEY_LENGTH: usize = 32;

        let mut rng = rand::thread_rng();
        (0..KEY_LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new("/etc/pi-door-client/secret.env")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_secret_from_env() {
        std::env::set_var("TEST_SECRET_KEY", "test_value");
        
        let store = SecretStore::new("/tmp/nonexistent");
        let result = store.load_secret("TEST_SECRET_KEY").unwrap();
        
        assert_eq!(result, Some("test_value".to_string()));
        
        std::env::remove_var("TEST_SECRET_KEY");
    }

    #[test]
    fn test_load_secret_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        fs::write(path, "TEST_KEY=test_value\nANOTHER_KEY=another_value\n").unwrap();
        
        let store = SecretStore::new(path);
        let result = store.load_secret("TEST_KEY").unwrap();
        
        assert_eq!(result, Some("test_value".to_string()));
    }

    #[test]
    fn test_save_and_load_secret() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let store = SecretStore::new(path);
        store.save_secret("NEW_KEY", "new_value").unwrap();
        
        let result = store.load_secret("NEW_KEY").unwrap();
        assert_eq!(result, Some("new_value".to_string()));
    }

    #[test]
    fn test_generate_api_key() {
        let key = SecretStore::generate_api_key();
        assert_eq!(key.len(), 32);
        assert!(key.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_rotate_jwt_token() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let store = SecretStore::new(path);
        
        // Save initial token
        store.save_secret("PI_CLIENT_JWT", "old_token").unwrap();
        
        // Rotate to new token
        store.rotate_jwt_token("new_token").unwrap();
        
        // Verify new token is active
        let current = store.load_jwt_token().unwrap();
        assert_eq!(current, Some("new_token".to_string()));
        
        // Verify old token is backed up
        let old = store.load_secret("PI_CLIENT_JWT_OLD").unwrap();
        assert_eq!(old, Some("old_token".to_string()));
    }
}
