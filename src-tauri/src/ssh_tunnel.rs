// SSH tunnel management for establishing reverse tunnels to the server
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::{AppHandle, Manager};

#[cfg(debug_assertions)]
const SSH_SERVER: &str = "tnnl.to";

#[cfg(not(debug_assertions))]
const SSH_SERVER: &str = "tnnl.to";

const SSH_USER: &str = "tnnl";
const SSH_KEY_FILENAME: &str = "id_ed25519";

/// SSH tunnel state
#[derive(Clone)]
pub struct SshTunnelState {
    ssh_process: Option<u32>, // Process ID
    remote_port: Option<u16>,
    local_port: Option<u16>,
}

/// Global SSH tunnel manager
pub struct SshTunnelManager {
    state: Arc<RwLock<SshTunnelState>>,
    ssh_key_path: PathBuf,
    ssh_pub_key_path: PathBuf,
}

impl SshTunnelManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Get home directory and create .tnnl folder there
        let home_dir = std::env::var("HOME")
            .map_err(|e| anyhow!("Failed to get HOME directory: {}", e))?;
        let tnnl_dir = PathBuf::from(home_dir).join(".tnnl");
        if !tnnl_dir.exists() {
            std::fs::create_dir_all(&tnnl_dir)?;
        }

        let ssh_key_path = tnnl_dir.join(SSH_KEY_FILENAME);
        let ssh_pub_key_path = tnnl_dir.join(format!("{}.pub", SSH_KEY_FILENAME));

        Ok(Self {
            state: Arc::new(RwLock::new(SshTunnelState {
                ssh_process: None,
                remote_port: None,
                local_port: None,
            })),
            ssh_key_path,
            ssh_pub_key_path,
        })
    }

    /// Generate SSH keypair if it doesn't exist
    pub fn ensure_ssh_keys(&self) -> Result<()> {
        if self.ssh_key_path.exists() && self.ssh_pub_key_path.exists() {
            println!("[SSH Tunnel] SSH keys already exist");
            return Ok(());
        }

        println!("[SSH Tunnel] Generating SSH keypair...");

        // Generate Ed25519 keypair
        let output = Command::new("ssh-keygen")
            .args(&[
                "-t", "ed25519",
                "-f", &self.ssh_key_path.to_string_lossy(),
                "-N", "", // No passphrase
                "-C", "tnnl@client", // Comment
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to generate SSH key: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        println!("[SSH Tunnel] SSH keypair generated successfully");
        Ok(())
    }

    /// Get the public key content
    pub fn get_public_key(&self) -> Result<String> {
        if !self.ssh_pub_key_path.exists() {
            return Err(anyhow!("SSH public key not found. Run ensure_ssh_keys() first."));
        }

        let content = std::fs::read_to_string(&self.ssh_pub_key_path)?;
        Ok(content.trim().to_string())
    }

    /// Establish SSH reverse tunnel
    /// Example: ssh -R remote_port:localhost:local_port -N tnnl@server
    pub async fn establish_tunnel(
        &self,
        remote_port: u16,
        local_port: u16,
    ) -> Result<()> {
        // Check if already connected
        {
            let mut state = self.state.write().await;
            if let Some(pid) = state.ssh_process {
                // Verify the process is actually running
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;

                    let pid_obj = Pid::from_raw(pid as i32);
                    // Signal 0 checks if process exists without sending a real signal
                    if kill(pid_obj, None).is_ok() {
                        return Err(anyhow!("SSH tunnel already active"));
                    }

                    // Process doesn't exist, clear stale state
                    eprintln!("[SSH Tunnel] Clearing stale tunnel state (PID {} not running)", pid);
                    state.ssh_process = None;
                    state.remote_port = None;
                    state.local_port = None;
                }

                #[cfg(windows)]
                {
                    // On Windows, just try to establish new tunnel
                    // TODO: Implement proper process checking on Windows
                    eprintln!("[SSH Tunnel] Clearing stale tunnel state (Windows)");
                    state.ssh_process = None;
                    state.remote_port = None;
                    state.local_port = None;
                }
            }
        }

        // Ensure SSH keys exist
        self.ensure_ssh_keys()?;

        println!("[SSH Tunnel] Establishing tunnel: remote_port={}, local_port={}", remote_port, local_port);

        // Build SSH command
        // ssh -R remote_port:localhost:local_port -N -o StrictHostKeyChecking=no -i key_path user@server
        eprintln!("[SSH Tunnel] SSH command: ssh -R {}:localhost:{} -N -o StrictHostKeyChecking=no -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -i {} {}@{}",
            remote_port, local_port, self.ssh_key_path.display(), SSH_USER, SSH_SERVER);

        let ssh_child = Command::new("ssh")
            .args(&[
                "-R", &format!("{}:localhost:{}", remote_port, local_port),
                "-N", // No remote command
                "-o", "StrictHostKeyChecking=no",
                "-o", "ServerAliveInterval=30",
                "-o", "ServerAliveCountMax=3",
                "-i", &self.ssh_key_path.to_string_lossy(),
                &format!("{}@{}", SSH_USER, SSH_SERVER),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                eprintln!("[SSH Tunnel] Failed to spawn SSH process: {}", e);
                e
            })?;

        let pid = ssh_child.id();
        println!("[SSH Tunnel] SSH process started with PID: {}", pid);

        // Update state
        {
            let mut state = self.state.write().await;
            state.ssh_process = Some(pid);
            state.remote_port = Some(remote_port);
            state.local_port = Some(local_port);
        }

        Ok(())
    }

    /// Close the SSH tunnel
    pub async fn close_tunnel(&self) -> Result<()> {
        let pid = {
            let mut state = self.state.write().await;
            let pid = state.ssh_process.take();
            state.remote_port = None;
            state.local_port = None;
            pid
        };

        if let Some(pid) = pid {
            println!("[SSH Tunnel] Closing SSH tunnel (PID: {})", pid);

            // Kill the SSH process
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(pid as i32);
                if let Err(e) = kill(pid, Signal::SIGTERM) {
                    eprintln!("[SSH Tunnel] Failed to kill SSH process: {}", e);
                }
            }

            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .output();
            }

            println!("[SSH Tunnel] SSH tunnel closed");
        }

        Ok(())
    }

    /// Check if tunnel is active
    pub async fn is_active(&self) -> bool {
        let state = self.state.read().await;
        state.ssh_process.is_some()
    }

    /// Get current tunnel info
    pub async fn get_tunnel_info(&self) -> Option<(u16, u16)> {
        let state = self.state.read().await;
        match (state.remote_port, state.local_port) {
            (Some(remote), Some(local)) => Some((remote, local)),
            _ => None,
        }
    }
}

// Global tunnel manager instance
static TUNNEL_MANAGER: once_cell::sync::Lazy<Arc<tokio::sync::Mutex<Option<SshTunnelManager>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(tokio::sync::Mutex::new(None)));

/// Initialize or get the global tunnel manager
pub async fn get_or_init_manager(app_handle: &AppHandle) -> Result<Arc<tokio::sync::Mutex<Option<SshTunnelManager>>>> {
    let manager_lock = TUNNEL_MANAGER.clone();
    let mut manager = manager_lock.lock().await;

    if manager.is_none() {
        *manager = Some(SshTunnelManager::new(app_handle)?);
    }

    Ok(manager_lock.clone())
}

/// Get the SSH public key
pub async fn get_ssh_public_key(app_handle: &AppHandle) -> Result<String> {
    let manager_lock = get_or_init_manager(app_handle).await?;
    let manager = manager_lock.lock().await;

    match manager.as_ref() {
        Some(mgr) => {
            mgr.ensure_ssh_keys()?;
            mgr.get_public_key()
        }
        None => Err(anyhow!("Tunnel manager not initialized")),
    }
}

/// Establish SSH tunnel
pub async fn establish_ssh_tunnel(
    app_handle: &AppHandle,
    remote_port: u16,
    local_port: u16,
) -> Result<()> {
    let manager_lock = get_or_init_manager(app_handle).await?;
    let manager = manager_lock.lock().await;

    match manager.as_ref() {
        Some(mgr) => mgr.establish_tunnel(remote_port, local_port).await,
        None => Err(anyhow!("Tunnel manager not initialized")),
    }
}

/// Close SSH tunnel
pub async fn close_ssh_tunnel(app_handle: &AppHandle) -> Result<()> {
    let manager_lock = get_or_init_manager(app_handle).await?;
    let manager = manager_lock.lock().await;

    match manager.as_ref() {
        Some(mgr) => mgr.close_tunnel().await,
        None => Err(anyhow!("Tunnel manager not initialized")),
    }
}

/// Check if tunnel is active
pub async fn is_tunnel_active(app_handle: &AppHandle) -> Result<bool> {
    let manager_lock = get_or_init_manager(app_handle).await?;
    let manager = manager_lock.lock().await;

    match manager.as_ref() {
        Some(mgr) => Ok(mgr.is_active().await),
        None => Ok(false),
    }
}
