use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};
use uuid::Uuid;

mod tunnel;
mod auth;
mod nginx;
mod db;
mod ssh_keys;

use tunnel::{Tunnel, TunnelManager};
use db::DbPool;

/// Represents a connected desktop app client
struct Client {
    id: Uuid,
    user_id: Option<Uuid>,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    tunnels: Vec<Tunnel>,
}

/// Global state shared across all connections
struct AppState {
    clients: RwLock<HashMap<Uuid, Client>>,
    tunnel_manager: TunnelManager,
    db_pool: DbPool,
    nginx_manager: nginx::NginxManager,
    auth_service: auth::AuthService,
}

impl AppState {
    fn new(db_pool: DbPool, jwt_secret: String) -> Arc<Self> {
        Arc::new(Self {
            clients: RwLock::new(HashMap::new()),
            tunnel_manager: TunnelManager::new(),
            db_pool,
            nginx_manager: nginx::NginxManager::new(),
            auth_service: auth::AuthService::new(jwt_secret),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    let addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set in .env (use Supabase JWT secret)");

    info!("Starting tnnl coordination server on {}", addr);

    // Initialize database connection pool
    info!("Connecting to database...");
    let db_pool = db::init_pool(&database_url).await?;
    info!("Database connected and migrations applied");

    // Initialize shared state
    let state = AppState::new(db_pool, jwt_secret);

    // Start WebSocket listener
    let listener = TcpListener::bind(&addr).await?;
    info!("WebSocket server listening on: {}", addr);

    while let Ok((stream, peer)) = listener.accept().await {
        info!("New connection from: {}", peer);
        tokio::spawn(handle_connection(stream, state.clone()));
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream, state: Arc<AppState>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Error during WebSocket handshake: {}", e);
            return;
        }
    };

    let client_id = Uuid::new_v4();
    info!("Client connected: {}", client_id);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Add client to state
    {
        let mut clients = state.clients.write().await;
        clients.insert(
            client_id,
            Client {
                id: client_id,
                user_id: None,
                sender: tx.clone(),
                tunnels: Vec::new(),
            },
        );
    }

    // Spawn task to send messages to client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                error!("Error sending message: {}", e);
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("Received message from {}: {}", client_id, text);
                handle_message(client_id, text, &state).await;
            }
            Ok(Message::Binary(_)) => {
                warn!("Received binary message from {}, ignoring", client_id);
            }
            Ok(Message::Close(_)) => {
                info!("Client {} disconnected", client_id);
                break;
            }
            Ok(Message::Ping(data)) => {
                info!("Received ping from {}", client_id);
                if let Some(client) = state.clients.read().await.get(&client_id) {
                    let _ = client.sender.send(Message::Pong(data));
                }
            }
            Ok(Message::Pong(_)) => {}
            Err(e) => {
                error!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup on disconnect
    info!("Cleaning up client {}", client_id);

    // Get client's tunnels before removing
    let tunnels_to_cleanup = {
        let clients = state.clients.read().await;
        clients.get(&client_id)
            .map(|client| client.tunnels.clone())
            .unwrap_or_default()
    };

    // Clean up each tunnel
    for tunnel in tunnels_to_cleanup {
        info!("Cleaning up tunnel: {}", tunnel.subdomain);

        // Remove nginx configuration
        if let Err(e) = state.nginx_manager.remove_tunnel_config(&tunnel.subdomain).await {
            error!("Failed to remove nginx config for {}: {}", tunnel.subdomain, e);
        }

        // Remove from tunnel manager
        if let Err(e) = state.tunnel_manager.remove_tunnel(&tunnel.subdomain).await {
            error!("Failed to remove tunnel {}: {}", tunnel.subdomain, e);
        }

        // Mark tunnel as inactive in database (don't delete, for history)
        if let Err(e) = db::delete_tunnel_record(&state.db_pool, &tunnel.subdomain).await {
            error!("Failed to delete tunnel record {}: {}", tunnel.subdomain, e);
        }

        info!("Tunnel {} cleaned up", tunnel.subdomain);
    }

    // Remove client from state
    {
        let mut clients = state.clients.write().await;
        clients.remove(&client_id);
    }

    send_task.abort();
    info!("Client {} removed and cleaned up", client_id);
}

async fn handle_message(client_id: Uuid, text: String, state: &Arc<AppState>) {
    // Parse message as JSON
    let msg: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse message: {}", e);
            send_error(client_id, "Invalid JSON", state).await;
            return;
        }
    };

    let msg_type = msg.get("type").and_then(|v| v.as_str());

    match msg_type {
        Some("auth") => {
            // Handle authentication
            info!("Authentication request from {}", client_id);

            let token = match msg.get("token").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => {
                    error!("Missing token in auth message");
                    send_error(client_id, "Missing token", state).await;
                    return;
                }
            };

            // Verify JWT token
            // Use insecure mode if DEV_MODE env var is set to "true"
            let use_dev_mode = std::env::var("DEV_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true";

            let (user_id, email) = if use_dev_mode {
                info!("Using DEV_MODE authentication (insecure)");
                match state.auth_service.verify_token_insecure(token) {
                    Ok((uid, em)) => (uid, em),
                    Err(e) => {
                        error!("Token verification failed: {}", e);
                        send_error(client_id, "Invalid token", state).await;
                        return;
                    }
                }
            } else {
                match state.auth_service.verify_supabase_token(token) {
                    Ok((uid, em)) => (uid, em),
                    Err(e) => {
                        error!("Token verification failed: {}", e);
                        send_error(client_id, "Invalid token", state).await;
                        return;
                    }
                }
            };

            // Store or update user in database
            if let Err(e) = db::get_or_create_user(&state.db_pool, user_id, &email).await {
                error!("Failed to store user: {}", e);
                send_error(client_id, "Database error", state).await;
                return;
            }

            // Update client with user_id
            {
                let mut clients = state.clients.write().await;
                if let Some(client) = clients.get_mut(&client_id) {
                    client.user_id = Some(user_id);
                }
            }

            // Send success response
            let response = serde_json::json!({
                "type": "auth_success",
                "user_id": user_id,
                "email": email
            });

            if let Some(client) = state.clients.read().await.get(&client_id) {
                let _ = client.sender.send(Message::Text(response.to_string()));
            }

            info!("Client {} authenticated as user {}", client_id, user_id);
        }
        Some("request_tunnel") => {
            // Handle tunnel request
            info!("Tunnel request from {}", client_id);

            // Get user_id from client
            let user_id = {
                let clients = state.clients.read().await;
                match clients.get(&client_id) {
                    Some(client) => match client.user_id {
                        Some(uid) => uid,
                        None => {
                            error!("Client {} not authenticated", client_id);
                            send_error(client_id, "Not authenticated", state).await;
                            return;
                        }
                    },
                    None => {
                        error!("Client {} not found", client_id);
                        return;
                    }
                }
            };

            // Get optional password from request
            let password = msg.get("password").and_then(|v| v.as_str()).map(String::from);

            // Create tunnel
            let tunnel = match state.tunnel_manager.create_random_tunnel(user_id, password).await {
                Ok(t) => t,
                Err(e) => {
                    error!("Failed to create tunnel: {}", e);
                    send_error(client_id, &format!("Tunnel creation failed: {}", e), state).await;
                    return;
                }
            };

            // Store tunnel in database
            if let Err(e) = db::create_tunnel_record(&state.db_pool, &tunnel).await {
                error!("Failed to store tunnel in database: {}", e);
                send_error(client_id, "Database error", state).await;
                return;
            }

            // Create Nginx configuration
            if let Err(e) = state.nginx_manager.create_tunnel_config(&tunnel).await {
                error!("Failed to create Nginx config: {}", e);
                send_error(client_id, &format!("Nginx configuration failed: {}", e), state).await;

                // Clean up tunnel
                let _ = state.tunnel_manager.remove_tunnel(&tunnel.subdomain).await;
                let _ = db::delete_tunnel_record(&state.db_pool, &tunnel.subdomain).await;
                return;
            }

            // Add tunnel to client's tunnel list
            {
                let mut clients = state.clients.write().await;
                if let Some(client) = clients.get_mut(&client_id) {
                    client.tunnels.push(tunnel.clone());
                }
            }

            // Send tunnel info to client
            let response = serde_json::json!({
                "type": "tunnel_assigned",
                "tunnel": {
                    "id": tunnel.id,
                    "subdomain": tunnel.subdomain,
                    "url": format!("https://{}.tnnl.to", tunnel.subdomain),
                    "port": tunnel.port,
                    "password": tunnel.password,
                    "created_at": tunnel.created_at.to_rfc3339()
                }
            });

            if let Some(client) = state.clients.read().await.get(&client_id) {
                let _ = client.sender.send(Message::Text(response.to_string()));
            }

            info!("Tunnel {} assigned to client {}", tunnel.subdomain, client_id);
        }
        Some("register_ssh_key") => {
            // Handle SSH key registration
            info!("SSH key registration from {}", client_id);

            // Get user_id from client
            let user_id = {
                let clients = state.clients.read().await;
                match clients.get(&client_id) {
                    Some(client) => match client.user_id {
                        Some(uid) => uid,
                        None => {
                            error!("Client {} not authenticated", client_id);
                            send_error(client_id, "Not authenticated", state).await;
                            return;
                        }
                    },
                    None => {
                        error!("Client {} not found", client_id);
                        return;
                    }
                }
            };

            // Get SSH public key from message
            let ssh_public_key = match msg.get("ssh_public_key").and_then(|v| v.as_str()) {
                Some(key) => key,
                None => {
                    error!("Missing ssh_public_key in message");
                    send_error(client_id, "Missing ssh_public_key", state).await;
                    return;
                }
            };

            // Validate SSH key
            if let Err(e) = ssh_keys::validate_ssh_public_key(ssh_public_key) {
                error!("Invalid SSH key: {}", e);
                send_error(client_id, &format!("Invalid SSH key: {}", e), state).await;
                return;
            }

            // Store SSH key in database
            if let Err(e) = db::store_ssh_public_key(&state.db_pool, user_id, ssh_public_key).await {
                error!("Failed to store SSH key: {}", e);
                send_error(client_id, "Failed to store SSH key", state).await;
                return;
            }

            // Add to authorized_keys file
            if let Err(e) = ssh_keys::add_ssh_key_to_authorized_keys(ssh_public_key).await {
                error!("Failed to add SSH key to authorized_keys: {}", e);
                send_error(client_id, "Failed to register SSH key", state).await;
                return;
            }

            // Send success response
            let response = serde_json::json!({
                "type": "ssh_key_registered",
                "success": true
            });

            if let Some(client) = state.clients.read().await.get(&client_id) {
                let _ = client.sender.send(Message::Text(response.to_string()));
            }

            info!("SSH key registered for user {}", user_id);
        }
        Some("heartbeat") => {
            // Respond to heartbeat
            if let Some(client) = state.clients.read().await.get(&client_id) {
                let response = serde_json::json!({
                    "type": "heartbeat_ack",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                let _ = client.sender.send(Message::Text(response.to_string()));
            }
        }
        _ => {
            warn!("Unknown message type: {:?}", msg_type);
            send_error(client_id, "Unknown message type", state).await;
        }
    }
}

/// Helper function to send error message to client
async fn send_error(client_id: Uuid, message: &str, state: &Arc<AppState>) {
    let error_msg = serde_json::json!({
        "type": "error",
        "message": message
    });

    if let Some(client) = state.clients.read().await.get(&client_id) {
        let _ = client.sender.send(Message::Text(error_msg.to_string()));
    }
}
