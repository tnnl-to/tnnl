use futures_util::{SinkExt, StreamExt};
use std::net::{SocketAddr, IpAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;
use once_cell::sync::Lazy;

/// Global WebSocket server state using tokio's async RwLock
static WS_STATE: Lazy<Arc<RwLock<Option<ServerState>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

struct ServerState {
    address: SocketAddr,
    frame_tx: broadcast::Sender<Vec<u8>>,
    shutdown_tx: broadcast::Sender<()>,
}

/// Start the WebSocket server on a specific port
pub async fn start_server(port: u16) -> Result<String, Box<dyn std::error::Error>> {
    // Check if already running - if so, force stop first
    {
        let state = WS_STATE.read().await;
        if state.is_some() {
            drop(state);
            let _ = stop_server().await;
            println!("[tnnl] Forced stop of existing WebSocket server");
            // Give OS time to release the port
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    let addr = format!("0.0.0.0:{}", port);

    // Try to bind - after force-stopping our state, the port should be free
    // If it's still in use after our cleanup, wait a bit for OS to release it
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("[tnnl] Port {} still in use after cleanup, waiting for OS to release...", port);

            // Wait for OS to release the port (our process stopped using it)
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            // Retry binding
            match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[tnnl] Port {} still in use after waiting. This may be an external process.", port);
                    return Err(format!("Port {} is in use. Please stop any other process using this port.", port).into());
                }
            }
        }
        Err(e) => return Err(e.into()),
    };

    let local_addr = listener.local_addr()?;

    println!("[tnnl] WebSocket server starting on {}", local_addr);

    // Create broadcast channel for frames (capacity: 2 frames buffered)
    let (frame_tx, _frame_rx) = broadcast::channel::<Vec<u8>>(2);

    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // Store server state
    {
        let mut state = WS_STATE.write().await;
        *state = Some(ServerState {
            address: local_addr,
            frame_tx: frame_tx.clone(),
            shutdown_tx: shutdown_tx.clone(),
        });
    }

    // Spawn server task
    tokio::spawn(async move {
        println!("[tnnl] WebSocket server listening...");

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            println!("[tnnl] New connection from: {}", peer_addr);
                            let frame_rx = frame_tx.subscribe();
                            tokio::spawn(handle_connection(stream, peer_addr, frame_rx));
                        }
                        Err(e) => {
                            eprintln!("[tnnl] Accept error: {}", e);
                            break;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    println!("[tnnl] Shutdown signal received, stopping listener");
                    break;
                }
            }
        }
        println!("[tnnl] WebSocket server task terminated");
    });

    Ok(format!("WebSocket server started on {}", local_addr))
}

/// Handle client messages
async fn handle_client_message(
    message: serde_json::Value,
    response_tx: tokio::sync::mpsc::Sender<String>,
) {
    let msg_type = message.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "get_apps" => {
            // Client requesting list of running apps
            println!("[tnnl] Client requested app list");

            let apps_opt = match crate::window_manager::get_running_applications() {
                Ok(apps) => Some(apps),
                Err(e) => {
                    eprintln!("[tnnl] Failed to get apps: {}", e);
                    None
                }
            };

            if let Some(apps) = apps_opt {
                let response = serde_json::json!({
                    "type": "apps",
                    "apps": apps
                });

                if let Ok(json_str) = serde_json::to_string(&response) {
                    let _ = response_tx.send(json_str).await;
                }
            }
        }
        "switch_app" => {
            // Client requesting to switch to a different app
            if let Some(bundle_id) = message.get("bundle_id").and_then(|v| v.as_str()) {
                println!("[tnnl] Client requested app switch to: {}", bundle_id);

                // Activate the app (synchronous call)
                let activation_result = crate::window_manager::activate_application(bundle_id);

                // Handle result outside the match to avoid Send issues
                if activation_result.is_ok() {
                    println!("[tnnl] Successfully switched to: {}", bundle_id);

                    // Switch to window-only capture mode for the new app
                    // Spawn a task to handle async operations
                    tokio::spawn(async move {
                        // Wait a moment for the app to become active
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                        // Get window bounds for the frontmost window
                        let window_bounds = crate::window_manager::get_frontmost_window();

                        if let Some((window_id, x, y, width, height)) = window_bounds {
                            println!("[tnnl] Found frontmost window: id={}, bounds=({}, {}, {}, {})",
                                window_id, x, y, width, height);

                            // Get app info for the name
                            if let Ok(Some(app)) = crate::window_manager::get_foreground_application() {
                                let app_name = app.app_name.clone();
                                println!("[tnnl] Switching to window capture for app: {}", app_name);

                                // Switch capture to crop to this window's bounds
                                // Extract success before any further awaits to avoid Send issues
                                let capture_success = {
                                    let capture_result = crate::screen_capture::set_capture_mode(
                                        crate::screen_capture::CaptureMode::Window {
                                            app_name: app_name.clone(),
                                            window_title: String::new(),
                                            crop_rect: Some((x, y, width, height)),
                                        }
                                    ).await;

                                    match capture_result {
                                        Ok(_) => true,
                                        Err(e) => {
                                            eprintln!("[tnnl] Failed to switch capture mode: {}", e);
                                            false
                                        }
                                    }
                                };

                                if capture_success {
                                    println!("[tnnl] ✓ Switched to window-only capture for {}", app_name);

                                    // Start focus observer to automatically update crop when user switches apps
                                    if let Err(e) = crate::window_manager::start_focus_observer().await {
                                        eprintln!("[tnnl] Failed to start focus observer: {}", e);
                                    }
                                }
                            }
                        } else {
                            println!("[tnnl] Could not get window bounds, keeping full display mode");
                        }
                    });
                } else if let Err(e) = activation_result {
                    eprintln!("[tnnl] Failed to switch app: {}", e);
                }
            }
        }
        "client_dimensions" => {
            // Client sending screen dimensions
            if let (Some(width), Some(height)) = (
                message.get("width").and_then(|v| v.as_f64()),
                message.get("height").and_then(|v| v.as_f64()),
            ) {
                println!(
                    "[tnnl] Client screen dimensions: {}x{}",
                    width, height
                );
                // TODO: Store client dimensions for future window resizing
            }
        }
        "mouse_move" => {
            // Client sending mouse movement
            if let (Some(x), Some(y), Some(width), Some(height)) = (
                message.get("x").and_then(|v| v.as_f64()),
                message.get("y").and_then(|v| v.as_f64()),
                message.get("client_width").and_then(|v| v.as_f64()),
                message.get("client_height").and_then(|v| v.as_f64()),
            ) {
                let (mac_x, mac_y) = crate::input_handler::map_coordinates(x, y, width, height);
                if let Err(e) = crate::input_handler::with_controller(|controller| {
                    controller.move_mouse(mac_x, mac_y)
                }) {
                    eprintln!("[tnnl] Mouse move failed: {}", e);
                }
            }
        }
        "mouse_click" => {
            // Client sending mouse click
            if let Some(button) = message.get("button").and_then(|v| v.as_str()) {
                let mouse_button = match button {
                    "left" => crate::input_handler::MouseButton::Left,
                    "right" => crate::input_handler::MouseButton::Right,
                    "middle" => crate::input_handler::MouseButton::Middle,
                    _ => return,
                };

                if let Err(e) = crate::input_handler::with_controller(|controller| {
                    controller.click(mouse_button)
                }) {
                    eprintln!("[tnnl] Mouse click failed: {}", e);
                }
            }
        }
        "mouse_scroll" => {
            // Client sending scroll event
            if let (Some(delta_x), Some(delta_y)) = (
                message.get("delta_x").and_then(|v| v.as_i64()),
                message.get("delta_y").and_then(|v| v.as_i64()),
            ) {
                if let Err(e) = crate::input_handler::with_controller(|controller| {
                    controller.scroll(delta_x as i32, delta_y as i32)
                }) {
                    eprintln!("[tnnl] Scroll failed: {}", e);
                }
            }
        }
        "send_key" => {
            // Client sending keyboard key press
            if let Some(key_code) = message.get("key_code").and_then(|v| v.as_u64()) {
                println!("[tnnl] Client requested key press: {}", key_code);
                if let Err(e) = crate::input_handler::with_controller(|controller| {
                    controller.send_key(key_code as u16)
                }) {
                    eprintln!("[tnnl] Key press failed: {}", e);
                }
            }
        }
        "send_key_combo" => {
            // Client sending keyboard combination
            if let Some(key_code) = message.get("key_code").and_then(|v| v.as_u64()) {
                let cmd = message.get("cmd").and_then(|v| v.as_bool()).unwrap_or(false);
                let shift = message.get("shift").and_then(|v| v.as_bool()).unwrap_or(false);
                let alt = message.get("alt").and_then(|v| v.as_bool()).unwrap_or(false);
                let ctrl = message.get("ctrl").and_then(|v| v.as_bool()).unwrap_or(false);

                println!("[tnnl] Client requested key combo: {} (cmd={}, shift={}, alt={}, ctrl={})",
                         key_code, cmd, shift, alt, ctrl);

                if let Err(e) = crate::input_handler::with_controller(|controller| {
                    controller.send_key_combination(key_code as u16, cmd, shift, alt, ctrl)
                }) {
                    eprintln!("[tnnl] Key combo failed: {}", e);
                }

                // Detect Cmd+` (key_code 50) for window switching within an app
                if cmd && key_code == 50 && !shift && !alt && !ctrl {
                    println!("[tnnl] Detected Cmd+` - refreshing window crop in 300ms");
                    tokio::spawn(async move {
                        // Wait for window switch animation
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                        // Refresh crop to the new focused window
                        if let Err(e) = crate::screen_capture::refresh_window_crop().await {
                            eprintln!("[tnnl] Failed to refresh window crop: {}", e);
                        }
                    });
                }
            }
        }
        "type_text" => {
            // Client sending text to type
            if let Some(text) = message.get("text").and_then(|v| v.as_str()) {
                println!("[tnnl] Client requested text input: {}", text);
                if let Err(e) = crate::input_handler::with_controller(|controller| {
                    controller.type_string(text)
                }) {
                    eprintln!("[tnnl] Text input failed: {}", e);
                }
            }
        }
        "send_key_batch" => {
            // Client sending atomic batch of key events
            if let Some(events_val) = message.get("events") {
                let txn_id = message.get("txn_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let mut response_to_send: Option<String> = None;
                match serde_json::from_value::<Vec<crate::input_handler::KeyEvent>>(events_val.clone()) {
                    Ok(events) => {
                        let result = crate::input_handler::with_controller(|controller| {
                            controller.send_key_events(&events)
                        });
                        // Build response synchronously to avoid holding non-Send errors across await
                        let ack_value = match result {
                            Ok(_) => serde_json::json!({
                                "type": "ack",
                                "ack_of": "send_key_batch",
                                "ok": true,
                                "txn_id": txn_id,
                            }),
                            Err(err) => {
                                eprintln!("[tnnl] Key batch failed: {}", err);
                                let err_msg = err.to_string();
                                serde_json::json!({
                                    "type": "ack",
                                    "ack_of": "send_key_batch",
                                    "ok": false,
                                    "error": err_msg,
                                    "txn_id": txn_id,
                                })
                            }
                        };
                        if let Ok(json_str) = serde_json::to_string(&ack_value) {
                            response_to_send = Some(json_str);
                        }
                    }
                    Err(err) => {
                        eprintln!("[tnnl] Invalid key batch payload: {}", err);
                        let ack_value = serde_json::json!({
                            "type": "ack",
                            "ack_of": "send_key_batch",
                            "ok": false,
                            "error": format!("invalid payload: {}", err),
                            "txn_id": txn_id,
                        });
                        if let Ok(json_str) = serde_json::to_string(&ack_value) {
                            response_to_send = Some(json_str);
                        }
                    }
                }
                if let Some(json_str) = response_to_send {
                    let _ = response_tx.send(json_str).await;
                }
            }
        }
        _ => {
            println!("[tnnl] Unknown message type: {}", msg_type);
        }
    }
}

/// Handle individual WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    mut frame_rx: broadcast::Receiver<Vec<u8>>,
) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("[tnnl] WebSocket handshake error: {}", e);
            return;
        }
    };

    println!("[tnnl] WebSocket connected: {}", peer_addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send welcome message
    if let Err(e) = ws_sender
        .send(Message::Text(
            r#"{"type":"welcome","message":"Connected to tnnl"}"#.to_string(),
        ))
        .await
    {
        eprintln!("[tnnl] Failed to send welcome: {}", e);
        return;
    }

    // Create channel for responses to send back to client
    let (response_tx, mut response_rx) = tokio::sync::mpsc::channel::<String>(32);

    // Spawn task to receive messages from client
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("[tnnl] Received from client: {}", text);

                    // Parse and handle JSON messages
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                        handle_client_message(value, response_tx.clone()).await;
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("[tnnl] Client closed connection");
                    break;
                }
                Err(e) => {
                    eprintln!("[tnnl] WebSocket receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Send frames and responses to client
    loop {
        tokio::select! {
            // Receive and send frames
            frame_result = frame_rx.recv() => {
                match frame_result {
                    Ok(frame_data) => {
                        // Send frame as binary WebSocket message with a small timeout to avoid blocking on slow clients
                        match tokio::time::timeout(tokio::time::Duration::from_millis(50), ws_sender.send(Message::Binary(frame_data))).await {
                            Ok(Ok(_)) => {},
                            Ok(Err(e)) => {
                                eprintln!("[tnnl] Failed to send frame: {}", e);
                                break;
                            }
                            Err(_) => {
                                // Timed out sending frame; drop this frame to keep pipeline moving
                                // Do not break; continue to next frame
                                // Optionally log occasionally
                                // eprintln!("[tnnl] Send timed out; dropping frame for {}", peer_addr);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        println!("[tnnl] Client lagging, skipped {} frames", skipped);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        println!("[tnnl] Frame channel closed");
                        break;
                    }
                }
            }
            // Receive and send text responses
            Some(response) = response_rx.recv() => {
                if let Err(e) = ws_sender.send(Message::Text(response)).await {
                    eprintln!("[tnnl] Failed to send response: {}", e);
                    break;
                }
            }
        }
    }

    // Clean up
    receiver_task.abort();
    println!("[tnnl] Client disconnected: {}", peer_addr);
}

/// Broadcast a frame to all connected clients
pub async fn broadcast_frame(frame_data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let state = WS_STATE.read().await;

    if let Some(server_state) = state.as_ref() {
        // Send to broadcast channel (non-blocking)
        match server_state.frame_tx.send(frame_data) {
            Ok(receiver_count) => {
                if receiver_count > 0 {
                    // Only log if there are actually connected clients
                    // println!("[tnnl] Frame broadcasted to {} clients", receiver_count);
                }
            }
            Err(_) => {
                // No receivers, that's okay
            }
        }
    }

    Ok(())
}

/// Get the local IP address
fn get_local_ip() -> Option<IpAddr> {
    use std::net::UdpSocket;

    // Connect to a public DNS server to get our local IP
    // This doesn't actually send any data
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip())
}

/// Get server information
pub async fn get_server_info() -> Result<ServerInfo, Box<dyn std::error::Error>> {
    let state = WS_STATE.read().await;

    match state.as_ref() {
        Some(server_state) => {
            // Replace 0.0.0.0 with actual local IP
            let display_addr = if server_state.address.ip().is_unspecified() {
                if let Some(local_ip) = get_local_ip() {
                    format!("{}:{}", local_ip, server_state.address.port())
                } else {
                    format!("{}", server_state.address)
                }
            } else {
                format!("{}", server_state.address)
            };

            Ok(ServerInfo {
                is_running: true,
                address: display_addr,
                client_count: server_state.frame_tx.receiver_count(),
            })
        }
        None => Ok(ServerInfo {
            is_running: false,
            address: "Not running".to_string(),
            client_count: 0,
        }),
    }
}

/// Stop the WebSocket server
pub async fn stop_server() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = WS_STATE.write().await;

    if let Some(server_state) = state.take() {
        // Send shutdown signal to terminate the listener task
        let _ = server_state.shutdown_tx.send(());
        println!("[tnnl] WebSocket server stopped");
        Ok(())
    } else {
        Err("WebSocket server not running".into())
    }
}

/// Clean up any orphaned processes using port 9001 from previous sessions
/// This is especially important after force quits or crashes
pub fn cleanup_orphaned_port_9001() -> Result<(), Box<dyn std::error::Error>> {
    println!("[WebSocket] Cleaning up orphaned processes on port 9001...");

    #[cfg(unix)]
    {
        use std::process::Command;

        // Find processes using port 9001
        let output = Command::new("lsof")
            .args(&["-ti", ":9001"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let pids: Vec<&str> = stdout.trim().lines().collect();

            if pids.is_empty() {
                println!("[WebSocket] No orphaned processes found on port 9001");
                return Ok(());
            }

            println!("[WebSocket] Found {} process(es) using port 9001", pids.len());

            for pid_str in pids {
                if let Ok(pid) = pid_str.parse::<i32>() {
                    println!("[WebSocket] Killing process PID: {}", pid);

                    #[cfg(target_os = "macos")]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        let pid_obj = Pid::from_raw(pid);
                        if let Err(e) = kill(pid_obj, Signal::SIGKILL) {
                            eprintln!("[WebSocket] Failed to kill PID {}: {}", pid, e);
                        } else {
                            println!("[WebSocket] ✓ Killed process PID: {}", pid);
                        }
                    }

                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = Command::new("kill")
                            .arg("-9")
                            .arg(pid.to_string())
                            .output();
                        println!("[WebSocket] ✓ Killed process PID: {}", pid);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use netstat and taskkill
        // TODO: Implement Windows cleanup
        eprintln!("[WebSocket] Port cleanup not yet implemented for Windows");
    }

    Ok(())
}

/// Server information struct
#[derive(Debug, serde::Serialize)]
pub struct ServerInfo {
    pub is_running: bool,
    pub address: String,
    pub client_count: usize,
}
