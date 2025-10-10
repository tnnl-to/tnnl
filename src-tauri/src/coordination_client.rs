use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use tauri::AppHandle;

#[cfg(debug_assertions)]
const COORDINATION_SERVER_URL: &str = "wss://tnnl.to";

#[cfg(not(debug_assertions))]
const COORDINATION_SERVER_URL: &str = "wss://tnnl.to";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub id: Uuid,
    pub subdomain: String,
    pub url: String,
    pub port: u16,
    pub password: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Authenticated,
    TunnelAssigned,
    Error(String),
}

#[derive(Clone)]
pub struct CoordinationClient {
    status: Arc<RwLock<ConnectionStatus>>,
    tunnel: Arc<RwLock<Option<TunnelInfo>>>,
    access_token: Arc<RwLock<Option<String>>>,
}

impl CoordinationClient {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            tunnel: Arc::new(RwLock::new(None)),
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to coordination server with authentication token
    pub async fn connect(&self, app_handle: AppHandle, access_token: String, password: Option<String>) -> Result<()> {
        // Write to a file to confirm this function is being called
        std::fs::write("/tmp/tnnl_connect_called.txt", format!("Connect called at {:?}\n", std::time::SystemTime::now())).ok();

        eprintln!("\n\n==> [Coordination] CONNECT FUNCTION CALLED\n");
        println!("\n\n==> [Coordination] CONNECT FUNCTION CALLED (stdout)\n");

        // Store token for reconnection
        *self.access_token.write().await = Some(access_token.clone());

        *self.status.write().await = ConnectionStatus::Connecting;

        eprintln!("==> [Coordination] Attempting to connect to: {}\n", COORDINATION_SERVER_URL);

        // Connect to WebSocket server with timeout
        let connect_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            connect_async(COORDINATION_SERVER_URL)
        ).await;

        let (ws_stream, response) = match connect_result {
            Ok(Ok((stream, resp))) => {
                eprintln!("==> [Coordination] Connection successful, response: {:?}\n", resp);
                (stream, resp)
            },
            Ok(Err(e)) => {
                eprintln!("==> [Coordination] WebSocket connection failed: {:?}\n", e);
                return Err(anyhow!("Failed to connect to coordination server: {}", e));
            },
            Err(_) => {
                eprintln!("==> [Coordination] Connection timeout after 10 seconds\n");
                return Err(anyhow!("Connection timeout"));
            }
        };

        *self.status.write().await = ConnectionStatus::Connected;
        println!("[Coordination] Connected to server");

        let (mut write, mut read) = ws_stream.split();

        // Send authentication message
        let auth_msg = serde_json::json!({
            "type": "auth",
            "token": access_token
        });

        write
            .send(Message::Text(auth_msg.to_string()))
            .await
            .map_err(|e| anyhow!("Failed to send auth message: {}", e))?;

        println!("[Coordination] Sent auth message");

        // Clone for the message handler
        let status = self.status.clone();
        let tunnel = self.tunnel.clone();
        let password_clone = password.clone();
        let app_handle_clone = app_handle.clone();

        // Spawn task to handle incoming messages
        tokio::spawn(async move {
            let mut authenticated = false;
            let mut write_handle = write;

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        println!("[Coordination] Received: {}", text);

                        // Parse message
                        let value: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("[Coordination] Failed to parse message: {}", e);
                                continue;
                            }
                        };

                        let msg_type = value.get("type").and_then(|v| v.as_str());

                        match msg_type {
                            Some("auth_success") => {
                                println!("[Coordination] Authentication successful");
                                *status.write().await = ConnectionStatus::Authenticated;
                                authenticated = true;

                                // Register SSH public key
                                let ssh_public_key = match crate::ssh_tunnel::get_ssh_public_key(&app_handle_clone).await {
                                    Ok(key) => key,
                                    Err(e) => {
                                        eprintln!("[Coordination] Failed to get SSH public key: {}", e);
                                        *status.write().await = ConnectionStatus::Error(format!("Failed to get SSH public key: {}", e));
                                        continue;
                                    }
                                };

                                let ssh_key_msg = serde_json::json!({
                                    "type": "register_ssh_key",
                                    "ssh_public_key": ssh_public_key
                                });

                                if let Err(e) = write_handle
                                    .send(Message::Text(ssh_key_msg.to_string()))
                                    .await
                                {
                                    eprintln!("[Coordination] Failed to register SSH key: {}", e);
                                    *status.write().await = ConnectionStatus::Error(format!("Failed to register SSH key: {}", e));
                                }

                                println!("[Coordination] Sent SSH key registration");
                            }
                            Some("ssh_key_registered") => {
                                println!("[Coordination] SSH key registered successfully");

                                // Request tunnel
                                let tunnel_request = if let Some(pwd) = &password_clone {
                                    serde_json::json!({
                                        "type": "request_tunnel",
                                        "password": pwd
                                    })
                                } else {
                                    serde_json::json!({
                                        "type": "request_tunnel"
                                    })
                                };

                                if let Err(e) = write_handle
                                    .send(Message::Text(tunnel_request.to_string()))
                                    .await
                                {
                                    eprintln!("[Coordination] Failed to request tunnel: {}", e);
                                    *status.write().await = ConnectionStatus::Error(format!("Failed to request tunnel: {}", e));
                                }

                                println!("[Coordination] Requested tunnel");
                            }
                            Some("tunnel_assigned") => {
                                println!("[Coordination] Tunnel assigned!");

                                if let Some(tunnel_data) = value.get("tunnel") {
                                    let tunnel_info: TunnelInfo = match serde_json::from_value(tunnel_data.clone()) {
                                        Ok(t) => t,
                                        Err(e) => {
                                            eprintln!("[Coordination] Failed to parse tunnel info: {}", e);
                                            continue;
                                        }
                                    };

                                    println!("[Coordination] Tunnel URL: {}", tunnel_info.url);

                                    // Start WebSocket server on port 9001 if not already running
                                    let local_port = 9001;
                                    println!("[Coordination] Starting WebSocket server on port {}...", local_port);
                                    let ws_result = crate::websocket_server::start_server(local_port).await
                                        .map_err(|e| e.to_string());
                                    if let Err(error_msg) = ws_result {
                                        eprintln!("[Coordination] Failed to start WebSocket server: {}", error_msg);
                                        *status.write().await = ConnectionStatus::Error(format!("Failed to start WebSocket server: {}", error_msg));
                                        continue;
                                    }
                                    println!("[Coordination] WebSocket server started on port {}", local_port);

                                    // Establish SSH tunnel
                                    let remote_port = tunnel_info.port;

                                    if let Err(e) = crate::ssh_tunnel::establish_ssh_tunnel(
                                        &app_handle_clone,
                                        remote_port,
                                        local_port
                                    ).await {
                                        eprintln!("[Coordination] Failed to establish SSH tunnel: {}", e);
                                        *status.write().await = ConnectionStatus::Error(format!("Failed to establish SSH tunnel: {}", e));
                                        continue;
                                    }

                                    println!("[Coordination] SSH tunnel established: {}:localhost:{}", remote_port, local_port);

                                    *tunnel.write().await = Some(tunnel_info);
                                    *status.write().await = ConnectionStatus::TunnelAssigned;
                                }
                            }
                            Some("error") => {
                                let error_msg = value
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error");
                                eprintln!("[Coordination] Server error: {}", error_msg);
                                *status.write().await = ConnectionStatus::Error(error_msg.to_string());
                            }
                            Some("heartbeat_ack") => {
                                // Heartbeat acknowledged, connection is alive
                            }
                            _ => {
                                println!("[Coordination] Unknown message type: {:?}", msg_type);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        println!("[Coordination] Server closed connection");
                        *status.write().await = ConnectionStatus::Disconnected;
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("[Coordination] WebSocket error: {}", e);
                        *status.write().await = ConnectionStatus::Error(format!("WebSocket error: {}", e));
                        break;
                    }
                }
            }

            *status.write().await = ConnectionStatus::Disconnected;
        });

        Ok(())
    }

    /// Get current connection status
    pub async fn get_status(&self) -> ConnectionStatus {
        self.status.read().await.clone()
    }

    /// Get assigned tunnel info
    pub async fn get_tunnel(&self) -> Option<TunnelInfo> {
        self.tunnel.read().await.clone()
    }

    /// Check if connected and tunnel is assigned
    pub async fn is_ready(&self) -> bool {
        matches!(
            *self.status.read().await,
            ConnectionStatus::TunnelAssigned
        )
    }

    /// Disconnect from coordination server
    pub async fn disconnect(&self) -> Result<()> {
        println!("[Coordination] Disconnecting...");

        // Reset all state
        *self.status.write().await = ConnectionStatus::Disconnected;
        *self.tunnel.write().await = None;

        println!("[Coordination] Disconnected and state cleared");
        Ok(())
    }
}

// Global coordination client instance
static COORDINATION_CLIENT: once_cell::sync::Lazy<Arc<Mutex<Option<CoordinationClient>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize or get the global coordination client
pub async fn get_or_init_client() -> Arc<Mutex<Option<CoordinationClient>>> {
    COORDINATION_CLIENT.clone()
}

/// Connect to coordination server
pub async fn connect_to_coordination(app_handle: AppHandle, access_token: String, password: Option<String>) -> Result<()> {
    let client = CoordinationClient::new();
    client.connect(app_handle, access_token, password).await?;

    let mut global_client = COORDINATION_CLIENT.lock().await;
    *global_client = Some(client);

    Ok(())
}

/// Get tunnel info from global client
pub async fn get_tunnel_info() -> Option<TunnelInfo> {
    let client_lock = COORDINATION_CLIENT.lock().await;
    if let Some(client) = client_lock.as_ref() {
        client.get_tunnel().await
    } else {
        None
    }
}

/// Get connection status from global client
pub async fn get_connection_status() -> ConnectionStatus {
    let client_lock = COORDINATION_CLIENT.lock().await;
    if let Some(client) = client_lock.as_ref() {
        client.get_status().await
    } else {
        ConnectionStatus::Disconnected
    }
}

/// Disconnect from coordination server and clean up
pub async fn disconnect_from_coordination(app_handle: &AppHandle) -> Result<()> {
    // Close SSH tunnel first
    if let Err(e) = crate::ssh_tunnel::close_ssh_tunnel(app_handle).await {
        eprintln!("[Coordination] Failed to close SSH tunnel: {}", e);
    }

    // Disconnect coordination client
    let client_lock = COORDINATION_CLIENT.lock().await;
    if let Some(client) = client_lock.as_ref() {
        client.disconnect().await?;
    }

    Ok(())
}
