//! tnnl - Remote desktop application
// Silence warnings from objc crate's old cfg attributes
#![allow(unexpected_cfgs)]

use tauri::{Manager, menu::{MenuBuilder, MenuItemBuilder}, tray::TrayIconBuilder};

mod screen_capture;
mod webrtc_peer;
mod websocket_server;
mod window_manager;
mod input_handler;
mod workos_auth;
mod coordination_client;
mod ssh_tunnel;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            start_screen_capture,
            stop_screen_capture,
            get_capture_status,
            get_displays,
            check_permissions,
            init_webrtc,
            create_webrtc_offer,
            set_webrtc_answer,
            get_webrtc_state,
            close_webrtc,
            start_websocket_server,
            stop_websocket_server,
            get_websocket_info,
            get_running_apps,
            get_foreground_app,
            focus_app,
            resize_window,
            mouse_move,
            mouse_click,
            mouse_scroll,
            check_accessibility_permission,
            request_accessibility_permission,
            send_key,
            send_key_combo,
            type_text,
            workos_send_magic_link,
            workos_verify_code,
            connect_to_coordination_server,
            get_coordination_status,
            get_tunnel_info,
            disconnect_tunnel,
            is_tunnel_active,
            show_and_activate_window,
        ])
        .setup(|app| {
            // Initialize input controller
            if let Err(e) = input_handler::init() {
                eprintln!("[tnnl] ✗ Failed to initialize input controller: {}", e);
            } else {
                println!("[tnnl] ✓ Input controller initialized");
            }

            // Prevent app from quitting when window is closed (for tray app)
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                #[cfg(target_os = "macos")]
                let app_handle = app.app_handle().clone();

                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Hide the window instead of closing it
                        let _ = window_clone.hide();
                        api.prevent_close();

                        // On macOS, hide from app switcher when window closes
                        #[cfg(target_os = "macos")]
                        {
                            use tauri::ActivationPolicy;
                            let _ = app_handle.set_activation_policy(ActivationPolicy::Prohibited);
                        }
                    }
                });
            }

            // Build tray menu
            let toggle_capture = MenuItemBuilder::with_id("toggle_capture", "Toggle Screen Capture").build(app)?;
            let toggle_websocket = MenuItemBuilder::with_id("toggle_websocket", "Disconnect Tunnel").build(app)?;
            let show_settings = MenuItemBuilder::with_id("show_settings", "Show Settings").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&toggle_capture)
                .item(&toggle_websocket)
                .separator()
                .item(&show_settings)
                .separator()
                .item(&quit)
                .build()?;

            // Build and setup tray icon
            println!("[tnnl] Building tray icon with menu...");

            // Use app icon from config
            let app_icon = app.default_window_icon().ok_or("No app icon found")?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app_icon.clone())
                .tooltip("tnnl - Remote Desktop")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    println!("[tnnl] Menu event: {:?}", event.id());

                    match event.id().as_ref() {
                        "toggle_capture" => {
                            tauri::async_runtime::spawn(async move {
                                // Check current status
                                let is_capturing = match screen_capture::get_status().await {
                                    Ok(status) => status.is_capturing,
                                    Err(e) => {
                                        eprintln!("[tnnl] ✗ Failed to get capture status: {}", e);
                                        return;
                                    }
                                };

                                if is_capturing {
                                    // Currently capturing, so stop
                                    match screen_capture::stop_capture().await {
                                        Ok(msg) => println!("[tnnl] ✓ {}", msg),
                                        Err(e) => eprintln!("[tnnl] ✗ Stop capture failed: {}", e),
                                    }
                                } else {
                                    // Not capturing, so start
                                    match screen_capture::start_capture().await {
                                        Ok(msg) => println!("[tnnl] ✓ {}", msg),
                                        Err(e) => eprintln!("[tnnl] ✗ Screen capture failed: {}", e),
                                    }
                                }
                            });
                        }
                        "toggle_websocket" => {
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                // Check if tunnel is active
                                let tunnel_info = coordination_client::get_tunnel_info().await;

                                if tunnel_info.is_some() {
                                    // Tunnel is active, disconnect it
                                    match coordination_client::disconnect_from_coordination(&app_clone).await {
                                        Ok(_) => println!("[tnnl] ✓ Tunnel disconnected"),
                                        Err(e) => eprintln!("[tnnl] ✗ Tunnel disconnect failed: {}", e),
                                    }
                                } else {
                                    // Tunnel not active, open settings window for user to connect
                                    println!("[tnnl] Opening settings to connect to tunnel...");
                                    if let Some(window) = app_clone.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.set_focus();

                                        // On macOS, show in app switcher when window opens
                                        #[cfg(target_os = "macos")]
                                        {
                                            use tauri::ActivationPolicy;
                                            let _ = app_clone.set_activation_policy(ActivationPolicy::Regular);
                                        }
                                    }
                                }
                            });
                        }
                        "show_settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();

                                // On macOS, show in app switcher when window opens
                                #[cfg(target_os = "macos")]
                                {
                                    use tauri::ActivationPolicy;
                                    let _ = app.set_activation_policy(ActivationPolicy::Regular);
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            println!("[tnnl] ✓ Tray icon created successfully");

            #[cfg(debug_assertions)]
            {
                let _window = app.get_webview_window("main").unwrap();
                // _window.open_devtools(); // Disabled to prevent layout issues

                // Auto-start in dev mode using tauri::async_runtime
                tauri::async_runtime::spawn(async move {
                    // Wait a bit for the app to fully initialize
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    println!("[tnnl] 🚀 Dev mode: Auto-starting screen capture and WebSocket server...");

                    // Start screen capture
                    match screen_capture::start_capture().await {
                        Ok(msg) => println!("[tnnl] ✓ {}", msg),
                        Err(e) => eprintln!("[tnnl] ✗ Screen capture failed: {}", e),
                    }

                    // Start WebSocket server on port 9001
                    match websocket_server::start_server(9001).await {
                        Ok(msg) => println!("[tnnl] ✓ {}", msg),
                        Err(e) => eprintln!("[tnnl] ✗ WebSocket server failed: {}", e),
                    }

                    println!("[tnnl] 🎉 Dev mode ready! Connect to ws://YOUR_IP:9001");
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            {
                if let tauri::RunEvent::Ready = event {
                    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        });
}

#[tauri::command]
async fn start_screen_capture() -> Result<String, String> {
    screen_capture::start_capture()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_screen_capture() -> Result<String, String> {
    screen_capture::stop_capture()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_capture_status() -> Result<screen_capture::CaptureStatus, String> {
    screen_capture::get_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_displays() -> Result<Vec<screen_capture::DisplayInfo>, String> {
    screen_capture::get_displays()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn check_permissions() -> Result<bool, String> {
    if !screen_capture::is_supported() {
        return Err("Screen capture is not supported on this platform".to_string());
    }
    Ok(screen_capture::has_permission())
}

// WebRTC commands
#[tauri::command]
async fn init_webrtc() -> Result<String, String> {
    webrtc_peer::init_peer_connection()
        .await
        .map_err(|e| e.to_string())?;
    Ok("WebRTC peer connection initialized".to_string())
}

#[tauri::command]
async fn create_webrtc_offer() -> Result<String, String> {
    webrtc_peer::create_offer()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_webrtc_answer(answer: String) -> Result<String, String> {
    webrtc_peer::set_remote_answer(answer)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Remote answer set successfully".to_string())
}

#[tauri::command]
async fn get_webrtc_state() -> Result<String, String> {
    webrtc_peer::get_connection_state()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn close_webrtc() -> Result<String, String> {
    webrtc_peer::close_peer_connection()
        .await
        .map_err(|e| e.to_string())?;
    Ok("WebRTC connection closed".to_string())
}

// WebSocket streaming commands
#[tauri::command]
async fn start_websocket_server(port: u16) -> Result<String, String> {
    websocket_server::start_server(port)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_websocket_server() -> Result<String, String> {
    websocket_server::stop_server()
        .await
        .map_err(|e| e.to_string())?;
    Ok("WebSocket server stopped".to_string())
}

#[tauri::command]
async fn get_websocket_info() -> Result<websocket_server::ServerInfo, String> {
    websocket_server::get_server_info()
        .await
        .map_err(|e| e.to_string())
}

// Window management commands
#[tauri::command]
fn get_running_apps() -> Result<Vec<window_manager::AppInfo>, String> {
    window_manager::get_running_applications()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_foreground_app() -> Result<Option<window_manager::AppInfo>, String> {
    window_manager::get_foreground_application()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_app(bundle_id: String) -> Result<String, String> {
    window_manager::activate_application(&bundle_id)
        .map_err(|e| e.to_string())?;
    Ok(format!("Activated app: {}", bundle_id))
}

#[tauri::command]
fn resize_window(bundle_id: String, width: f64, height: f64) -> Result<String, String> {
    window_manager::resize_app_window(&bundle_id, width, height)
        .map_err(|e| e.to_string())?;
    Ok(format!("Resized window for {}: {}x{}", bundle_id, width, height))
}

// Input control commands
#[tauri::command]
fn mouse_move(x: f64, y: f64, client_width: f64, client_height: f64) -> Result<String, String> {
    // Map client coordinates to Mac screen coordinates
    let (mac_x, mac_y) = input_handler::map_coordinates(x, y, client_width, client_height);

    input_handler::with_controller(|controller| {
        controller.move_mouse(mac_x, mac_y)
    })
    .map_err(|e| e.to_string())?;

    Ok("Mouse moved".to_string())
}

#[tauri::command]
fn mouse_click(button: String) -> Result<String, String> {
    let mouse_button = match button.as_str() {
        "left" => input_handler::MouseButton::Left,
        "right" => input_handler::MouseButton::Right,
        "middle" => input_handler::MouseButton::Middle,
        _ => return Err("Invalid button type".to_string()),
    };

    input_handler::with_controller(|controller| {
        controller.click(mouse_button)
    })
    .map_err(|e| e.to_string())?;

    Ok(format!("{} click", button))
}

#[tauri::command]
fn mouse_scroll(delta_x: i32, delta_y: i32) -> Result<String, String> {
    input_handler::with_controller(|controller| {
        controller.scroll(delta_x, delta_y)
    })
    .map_err(|e| e.to_string())?;

    Ok("Scrolled".to_string())
}

#[tauri::command]
fn check_accessibility_permission() -> Result<bool, String> {
    Ok(input_handler::has_accessibility_permission())
}

#[tauri::command]
fn request_accessibility_permission() -> Result<String, String> {
    input_handler::request_accessibility_permission()
        .map_err(|e| e.to_string())?;
    Ok("Opened System Settings. Please grant Accessibility permission to this app.".to_string())
}

#[tauri::command]
fn send_key(key_code: u16) -> Result<String, String> {
    input_handler::with_controller(|controller| {
        controller.send_key(key_code)
    })
    .map_err(|e| e.to_string())?;

    Ok(format!("Sent key: {}", key_code))
}

#[tauri::command]
fn send_key_combo(key_code: u16, cmd: bool, shift: bool, alt: bool, ctrl: bool) -> Result<String, String> {
    input_handler::with_controller(|controller| {
        controller.send_key_combination(key_code, cmd, shift, alt, ctrl)
    })
    .map_err(|e| e.to_string())?;

    Ok("Sent key combination".to_string())
}

#[tauri::command]
fn type_text(text: String) -> Result<String, String> {
    input_handler::with_controller(|controller| {
        controller.type_string(&text)
    })
    .map_err(|e| e.to_string())?;

    Ok(format!("Typed: {}", text))
}

// WorkOS authentication commands
#[tauri::command]
async fn workos_send_magic_link(email: String) -> Result<String, String> {
    workos_auth::send_magic_link(email).await
}

#[tauri::command]
async fn workos_verify_code(code: String, auth_id: String) -> Result<workos_auth::VerifyCodeResponse, String> {
    workos_auth::verify_magic_code(code, auth_id).await
}

// Coordination server commands
#[tauri::command]
async fn connect_to_coordination_server(app: tauri::AppHandle, access_token: String, password: Option<String>) -> Result<String, String> {
    std::fs::write("/tmp/tauri_command_called.txt", format!("Tauri command called at {:?}\n", std::time::SystemTime::now())).ok();
    eprintln!("!!! TAURI COMMAND CALLED !!!");

    // Disconnect first if already connected
    eprintln!("[Connect] Checking for existing connection...");
    if let Err(e) = coordination_client::disconnect_from_coordination(&app).await {
        eprintln!("[Connect] Warning: Failed to disconnect existing connection: {}", e);
    }

    eprintln!("[Connect] Starting new connection...");
    coordination_client::connect_to_coordination(app, access_token, password)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Connected to coordination server".to_string())
}

#[tauri::command]
async fn get_coordination_status() -> Result<String, String> {
    let status = coordination_client::get_connection_status().await;
    let status_str = match status {
        coordination_client::ConnectionStatus::Disconnected => "disconnected",
        coordination_client::ConnectionStatus::Connecting => "connecting",
        coordination_client::ConnectionStatus::Connected => "connected",
        coordination_client::ConnectionStatus::Authenticated => "authenticated",
        coordination_client::ConnectionStatus::TunnelAssigned => "tunnel_assigned",
        coordination_client::ConnectionStatus::Error(ref e) => return Err(e.clone()),
    };
    Ok(status_str.to_string())
}

#[tauri::command]
async fn get_tunnel_info() -> Result<Option<coordination_client::TunnelInfo>, String> {
    Ok(coordination_client::get_tunnel_info().await)
}

#[tauri::command]
async fn disconnect_tunnel(app: tauri::AppHandle) -> Result<String, String> {
    coordination_client::disconnect_from_coordination(&app)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Tunnel disconnected".to_string())
}

#[tauri::command]
async fn is_tunnel_active(app: tauri::AppHandle) -> Result<bool, String> {
    ssh_tunnel::is_tunnel_active(&app)
        .await
        .map_err(|e| e.to_string())
}

// Window management command
#[tauri::command]
async fn show_and_activate_window(app: tauri::AppHandle) -> Result<String, String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        // On macOS, show in app switcher when window opens
        #[cfg(target_os = "macos")]
        {
            use tauri::ActivationPolicy;
            app.set_activation_policy(ActivationPolicy::Regular)
                .map_err(|e| e.to_string())?;
        }

        Ok("Window shown and activated".to_string())
    } else {
        Err("Window not found".to_string())
    }
}
