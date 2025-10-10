use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use image::ImageEncoder;
use std::io::Cursor;
use once_cell::sync::Lazy;
use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::{Frame, FrameType},
    Target,
};
use core_graphics::display::CGDisplay;

/// Global capture state using tokio's async RwLock
static CAPTURE_STATE: Lazy<Arc<RwLock<Option<CaptureSession>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Capture mode: full display or specific window with crop
#[derive(Debug, Clone)]
pub enum CaptureMode {
    FullDisplay,
    Window {
        app_name: String,
        window_title: String,
        crop_rect: Option<(f64, f64, f64, f64)>, // (x, y, width, height) in screen coordinates
    },
}

/// Represents an active capture session
struct CaptureSession {
    start_time: Instant,
    frame_count: u64,
    is_running: bool,
    mode: CaptureMode,
    capturer: Option<Arc<parking_lot::Mutex<Capturer>>>,
    crop_rect: Option<(f64, f64, f64, f64)>, // Window crop bounds if in Window mode
}

/// Information about available displays
#[derive(Debug, serde::Serialize, Clone)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Check if screen capture is supported on this platform
pub fn is_supported() -> bool {
    // scap works on all platforms
    true
}

/// Check if we have screen recording permission (macOS specific)
pub fn has_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        scap::has_permission()
    }

    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Request screen recording permission from the user
pub fn request_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        scap::request_permission();
    }

    has_permission()
}

/// Get information about all available displays
pub async fn get_displays() -> Result<Vec<DisplayInfo>, Box<dyn std::error::Error>> {
    let targets = scap::get_all_targets();

    let displays: Vec<DisplayInfo> = targets.iter()
        .filter_map(|target| {
            // scap targets are opaque, so we'll just create placeholder display info
            // In reality, scap doesn't expose target details before capture
            Some(DisplayInfo {
                id: format!("display_{}", targets.len()),
                name: "Display".to_string(),
                width: 1920,
                height: 1080,
                is_primary: true,
            })
        })
        .take(1) // Just return one display for now
        .collect();

    Ok(displays)
}

/// Start screen capture using the primary display
pub async fn start_capture() -> Result<String, Box<dyn std::error::Error>> {
    start_capture_with_mode(CaptureMode::FullDisplay).await
}

/// Start screen capture with a specific mode
pub async fn start_capture_with_mode(mode: CaptureMode) -> Result<String, Box<dyn std::error::Error>> {
    // Check if already capturing - if so, force stop first
    {
        let state = CAPTURE_STATE.read().await;
        if let Some(session) = state.as_ref() {
            if session.is_running {
                drop(state);
                let _ = stop_capture().await;
                println!("[tnnl] Forced stop of existing capture session");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    // Get available targets
    let targets = scap::get_all_targets();

    if targets.is_empty() {
        return Err("No capture targets available. On macOS, please grant Screen Recording permission in System Settings > Privacy & Security > Screen Recording".into());
    }

    println!("[tnnl] Found {} capture targets", targets.len());

    // Get primary display from targets (usually first display in list)
    let primary_display = targets.iter()
        .find(|t| matches!(t, Target::Display(_)))
        .cloned();

    // Extract crop rectangle from mode (if window mode)
    let crop_rect = match &mode {
        CaptureMode::FullDisplay => {
            println!("[tnnl] Full display mode - no cropping");
            None
        }
        CaptureMode::Window { app_name, crop_rect, .. } => {
            println!("[tnnl] Window mode for: {}", app_name);
            if let Some(rect) = crop_rect {
                println!("[tnnl] Crop rect: x={}, y={}, w={}, h={}", rect.0, rect.1, rect.2, rect.3);
            } else {
                println!("[tnnl] No crop rect provided, will capture full display");
            }
            *crop_rect
        }
    };

    // Always use display capture (stable, no crashes)
    let target = primary_display.clone();

    // Create capturer with options
    let options = Options {
        fps: 10, // Start conservative, can increase later
        target,
        show_cursor: true,
        show_highlight: false,
        excluded_targets: None,
        output_type: FrameType::BGRAFrame,
        output_resolution: Resolution::_1080p, // Increased from 720p for better quality
        crop_area: None,
    };

    let mut capturer = Capturer::build(options)?;
    capturer.start_capture();

    let capturer_arc = Arc::new(parking_lot::Mutex::new(capturer));

    // Store the session
    let session = CaptureSession {
        start_time: Instant::now(),
        frame_count: 0,
        is_running: true,
        mode: mode.clone(),
        capturer: Some(capturer_arc.clone()),
        crop_rect,
    };

    {
        let mut state = CAPTURE_STATE.write().await;
        *state = Some(session);
    }

    // Start background capture task
    start_capture_loop(capturer_arc);

    Ok(format!("Screen capture started successfully with {:?}", mode))
}

/// Background task that continuously captures frames
fn start_capture_loop(capturer: Arc<parking_lot::Mutex<Capturer>>) {
    tokio::task::spawn(async move {
        println!("[tnnl] Starting scap capture loop at 10 FPS");

        loop {
            // Check if we should stop
            {
                let state = CAPTURE_STATE.read().await;
                match state.as_ref() {
                    Some(session) if !session.is_running => {
                        println!("[tnnl] Capture loop stopped");
                        break;
                    }
                    None => {
                        println!("[tnnl] Capture loop stopped (no session)");
                        break;
                    }
                    _ => {} // Continue capturing
                }
            }

            // Capture a frame
            let frame_result = {
                let mut cap = capturer.lock();
                cap.get_next_frame()
            };

            match frame_result {
                Ok(frame) => {
                    // Frame captured successfully, get crop rect from session
                    let crop_rect = {
                        let mut state = CAPTURE_STATE.write().await;
                        if let Some(session) = state.as_mut() {
                            session.frame_count += 1;
                            session.crop_rect
                        } else {
                            None
                        }
                    }; // Release lock before encoding

                    // Convert frame to JPEG (with optional cropping) and broadcast
                    if let Ok(jpeg_data) = frame_to_jpeg(&frame, crop_rect, 90) {
                        let _ = crate::websocket_server::broadcast_frame(jpeg_data).await;
                    }
                }
                Err(e) => {
                    // Log error but don't stop - might be transient
                    if !e.to_string().contains("timeout") {
                        eprintln!("[tnnl] Frame capture error: {}", e);
                    }
                }
            }

            // Target 10 FPS (100ms delay)
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
}

/// Stop screen capture
pub async fn stop_capture() -> Result<String, Box<dyn std::error::Error>> {
    let mut state = CAPTURE_STATE.write().await;

    match state.take() {
        Some(session) => {
            let elapsed = session.start_time.elapsed();
            let fps = if elapsed.as_secs() > 0 {
                session.frame_count / elapsed.as_secs()
            } else {
                0
            };

            println!("[tnnl] Screen capture stopped");
            println!(
                "[tnnl] Captured {} frames in {:.2}s (avg {:.1} fps)",
                session.frame_count,
                elapsed.as_secs_f64(),
                fps
            );

            Ok(format!(
                "Screen capture stopped. Captured {} frames",
                session.frame_count
            ))
        }
        None => Err("Screen capture is not running".into()),
    }
}

/// Switch capture mode (e.g., from full display to window or vice versa)
/// This stops the current capture and starts a new one with the specified mode
pub async fn set_capture_mode(mode: CaptureMode) -> Result<(), Box<dyn std::error::Error>> {
    println!("[tnnl] Switching capture mode to: {:?}", mode);

    // Restart capture with new mode
    start_capture_with_mode(mode).await?;

    Ok(())
}

/// Refresh window crop bounds for the current foreground window
/// This updates the crop rectangle without restarting capture (efficient)
pub async fn refresh_window_crop() -> Result<(), Box<dyn std::error::Error>> {
    // Get current frontmost window bounds
    let window_bounds = crate::window_manager::get_frontmost_window();

    if let Some((window_id, x, y, width, height)) = window_bounds {
        println!("[tnnl] Refreshing crop to window: id={}, bounds=({}, {}, {}, {})",
            window_id, x, y, width, height);

        // Update the session's crop_rect
        let mut state = CAPTURE_STATE.write().await;
        if let Some(session) = state.as_mut() {
            session.crop_rect = Some((x, y, width, height));
            println!("[tnnl] ✓ Crop bounds updated");
        }
    } else {
        println!("[tnnl] Could not get window bounds for refresh");
    }

    Ok(())
}

/// Get the current capture status
pub async fn get_status() -> Result<CaptureStatus, Box<dyn std::error::Error>> {
    let state = CAPTURE_STATE.read().await;

    match state.as_ref() {
        Some(session) if session.is_running => {
            let elapsed = session.start_time.elapsed();
            let fps = if elapsed.as_secs() > 0 {
                session.frame_count / elapsed.as_secs()
            } else {
                0
            };

            Ok(CaptureStatus {
                is_capturing: true,
                frame_count: session.frame_count,
                elapsed_seconds: elapsed.as_secs_f64(),
                average_fps: fps,
            })
        }
        _ => Ok(CaptureStatus {
            is_capturing: false,
            frame_count: 0,
            elapsed_seconds: 0.0,
            average_fps: 0,
        }),
    }
}

/// Capture status information
#[derive(Debug, serde::Serialize)]
pub struct CaptureStatus {
    pub is_capturing: bool,
    pub frame_count: u64,
    pub elapsed_seconds: f64,
    pub average_fps: u64,
}

/// Get the primary display resolution
fn get_display_resolution() -> (f64, f64) {
    unsafe {
        let display = CGDisplay::main();
        let width = display.pixels_wide() as f64;
        let height = display.pixels_high() as f64;
        (width, height)
    }
}

/// Convert scap Frame to JPEG bytes with optional cropping
fn frame_to_jpeg(
    frame: &Frame,
    crop_rect: Option<(f64, f64, f64, f64)>, // (x, y, width, height) in screen coordinates
    quality: u8
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    match frame {
        Frame::BGRA(bgra_frame) => {
            let capture_width = bgra_frame.width as u32;
            let capture_height = bgra_frame.height as u32;

            // If crop rect provided, crop the frame
            let (final_width, final_height, rgb_buffer) = if let Some((crop_x, crop_y, crop_w, crop_h)) = crop_rect {
                // Get display resolution and calculate scale factor
                let (display_w, display_h) = get_display_resolution();
                let scale_x = capture_width as f64 / display_w;
                let scale_y = capture_height as f64 / display_h;

                // Scale crop bounds to capture coordinates
                let scaled_x = (crop_x * scale_x).max(0.0) as u32;
                let scaled_y = (crop_y * scale_y).max(0.0) as u32;
                let scaled_w = (crop_w * scale_x).min((capture_width - scaled_x) as f64) as u32;
                let scaled_h = (crop_h * scale_y).min((capture_height - scaled_y) as f64) as u32;

                println!("[tnnl] Cropping: display {}x{} → capture {}x{}, window ({},{},{},{}) → ({},{},{},{})",
                    display_w, display_h, capture_width, capture_height,
                    crop_x, crop_y, crop_w, crop_h,
                    scaled_x, scaled_y, scaled_w, scaled_h);

                // Crop and convert BGRA to RGB
                let mut rgb = Vec::with_capacity((scaled_w * scaled_h * 3) as usize);

                for y in scaled_y..(scaled_y + scaled_h) {
                    for x in scaled_x..(scaled_x + scaled_w) {
                        let index = ((y * capture_width + x) * 4) as usize;
                        if index + 3 < bgra_frame.data.len() {
                            rgb.push(bgra_frame.data[index + 2]); // R
                            rgb.push(bgra_frame.data[index + 1]); // G
                            rgb.push(bgra_frame.data[index + 0]); // B
                        }
                    }
                }

                (scaled_w, scaled_h, rgb)
            } else {
                // No cropping, convert entire frame from BGRA to RGB
                let mut rgb = Vec::with_capacity((capture_width * capture_height * 3) as usize);

                for pixel in bgra_frame.data.chunks(4) {
                    rgb.push(pixel[2]); // R
                    rgb.push(pixel[1]); // G
                    rgb.push(pixel[0]); // B
                }

                (capture_width, capture_height, rgb)
            };

            // Encode as JPEG
            let mut jpeg_data = Cursor::new(Vec::new());
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, quality);
            encoder.write_image(
                &rgb_buffer,
                final_width,
                final_height,
                image::ExtendedColorType::Rgb8,
            )?;

            Ok(jpeg_data.into_inner())
        }
        _ => Err("Unsupported frame type (expected BGRA)".into())
    }
}
