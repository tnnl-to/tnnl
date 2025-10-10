// SSH key management for tunnel authentication
use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const AUTHORIZED_KEYS_PATH: &str = "/home/tnnl/.ssh/authorized_keys";

/// Validate SSH public key format
/// Returns true if the key appears to be a valid SSH public key
pub fn validate_ssh_public_key(key: &str) -> Result<()> {
    let key = key.trim();

    // Check if empty
    if key.is_empty() {
        return Err(anyhow!("SSH key cannot be empty"));
    }

    // SSH public keys typically start with ssh-rsa, ssh-ed25519, ssh-dss, or ecdsa-sha2-*
    let valid_prefixes = ["ssh-rsa", "ssh-ed25519", "ssh-dss", "ecdsa-sha2-"];

    if !valid_prefixes.iter().any(|prefix| key.starts_with(prefix)) {
        return Err(anyhow!("Invalid SSH key format. Must start with ssh-rsa, ssh-ed25519, ssh-dss, or ecdsa-sha2-*"));
    }

    // Check that it has at least 2 space-separated parts (type and key data)
    let parts: Vec<&str> = key.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid SSH key format. Must contain at least key type and key data"));
    }

    // Basic length check - SSH keys are typically quite long
    // Ed25519 keys are around 80-100 chars, RSA keys are 300+
    if key.len() < 80 {
        return Err(anyhow!("SSH key appears too short to be valid"));
    }

    Ok(())
}

/// Add SSH public key to authorized_keys file
/// This allows the user to establish SSH tunnels
pub async fn add_ssh_key_to_authorized_keys(public_key: &str) -> Result<()> {
    // Validate key first
    validate_ssh_public_key(public_key)?;

    // In development mode, skip actual file operations
    #[cfg(debug_assertions)]
    {
        println!("[Dev Mode] Would add SSH key to authorized_keys: {}", public_key);
        return Ok(());
    }

    #[cfg(not(debug_assertions))]
    {
        // Ensure the .ssh directory exists
        let ssh_dir = Path::new(AUTHORIZED_KEYS_PATH).parent()
            .ok_or_else(|| anyhow!("Invalid authorized_keys path"))?;

        if !ssh_dir.exists() {
            fs::create_dir_all(ssh_dir).await?;
            // Set proper permissions (700 for .ssh directory)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(ssh_dir, std::fs::Permissions::from_mode(0o700)).await?;
            }
        }

        // Read existing authorized_keys if it exists
        let existing_keys = if Path::new(AUTHORIZED_KEYS_PATH).exists() {
            fs::read_to_string(AUTHORIZED_KEYS_PATH).await?
        } else {
            String::new()
        };

        // Check if this key is already present
        if existing_keys.lines().any(|line| line.trim() == public_key.trim()) {
            // Key already exists, no need to add it again
            return Ok(());
        }

        // Append the new key
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(AUTHORIZED_KEYS_PATH)
            .await?;

        // Ensure there's a newline before the key if file isn't empty
        let prefix = if existing_keys.is_empty() || existing_keys.ends_with('\n') {
            ""
        } else {
            "\n"
        };

        file.write_all(format!("{}{}\n", prefix, public_key.trim()).as_bytes()).await?;
        file.flush().await?;

        // Set proper permissions (600 for authorized_keys)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(AUTHORIZED_KEYS_PATH, std::fs::Permissions::from_mode(0o600)).await?;
        }

        Ok(())
    }
}

/// Remove SSH public key from authorized_keys file
/// Used for cleanup when a user is deleted
#[allow(dead_code)]
pub async fn remove_ssh_key_from_authorized_keys(public_key: &str) -> Result<()> {
    // In development mode, skip actual file operations
    #[cfg(debug_assertions)]
    {
        println!("[Dev Mode] Would remove SSH key from authorized_keys: {}", public_key);
        return Ok(());
    }

    #[cfg(not(debug_assertions))]
    {
        if !Path::new(AUTHORIZED_KEYS_PATH).exists() {
            // File doesn't exist, nothing to remove
            return Ok(());
        }

        // Read all keys
        let contents = fs::read_to_string(AUTHORIZED_KEYS_PATH).await?;

        // Filter out the key to remove
        let new_contents: String = contents
            .lines()
            .filter(|line| line.trim() != public_key.trim())
            .collect::<Vec<&str>>()
            .join("\n");

        // Write back the filtered keys
        fs::write(AUTHORIZED_KEYS_PATH, new_contents.as_bytes()).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ssh_public_key() {
        // Valid keys
        assert!(validate_ssh_public_key("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC... user@host").is_ok());
        assert!(validate_ssh_public_key("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMv... user@host").is_ok());

        // Invalid keys
        assert!(validate_ssh_public_key("").is_err());
        assert!(validate_ssh_public_key("not-an-ssh-key").is_err());
        assert!(validate_ssh_public_key("ssh-rsa").is_err()); // Too short
        assert!(validate_ssh_public_key("invalid-prefix AAAAB3NzaC1yc2E...").is_err());
    }
}
