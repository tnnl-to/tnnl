use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use core_graphics::window::CGWindowID;
use core_foundation::base::TCFType;
use core_foundation::number::CFNumber;
use core_foundation::dictionary::CFDictionary;
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::string::CFString;
use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

/// Information about a running application
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppInfo {
    pub bundle_id: String,
    pub app_name: String,
    pub process_id: i32,
    pub is_active: bool,
    pub icon_base64: Option<String>,
}

/// Get all running applications
pub fn get_running_applications() -> Result<Vec<AppInfo>, Box<dyn std::error::Error>> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Get shared workspace
        let workspace_class = class!(NSWorkspace);
        let workspace: id = msg_send![workspace_class, sharedWorkspace];

        // Get running applications
        let running_apps: id = msg_send![workspace, runningApplications];
        let count: usize = msg_send![running_apps, count];

        let mut apps = Vec::new();

        for i in 0..count {
            let app: id = msg_send![running_apps, objectAtIndex: i];

            // Get bundle identifier
            let bundle_id_ns: id = msg_send![app, bundleIdentifier];
            if bundle_id_ns == nil {
                continue;
            }

            let bundle_id = nsstring_to_string(bundle_id_ns);

            // Check activation policy - only show regular apps (like Cmd+Tab)
            let policy: i64 = msg_send![app, activationPolicy];
            // NSApplicationActivationPolicyRegular = 0 (regular apps)
            // NSApplicationActivationPolicyAccessory = 1 (menu bar apps - skip)
            // NSApplicationActivationPolicyProhibited = 2 (background only - skip)
            if policy != 0 {
                continue; // Skip non-regular apps
            }

            // Skip system processes and background apps
            if should_skip_app(&bundle_id) {
                continue;
            }

            // Get localized name
            let name_ns: id = msg_send![app, localizedName];
            let app_name = if name_ns != nil {
                nsstring_to_string(name_ns)
            } else {
                bundle_id.clone()
            };

            // Get process ID
            let pid: i32 = msg_send![app, processIdentifier];

            // Check if active/foreground
            let is_active: bool = msg_send![app, isActive];

            // Get icon (as base64 PNG for transport)
            let icon_base64 = get_app_icon_base64(app);

            apps.push(AppInfo {
                bundle_id,
                app_name,
                process_id: pid,
                is_active,
                icon_base64,
            });
        }

        Ok(apps)
    }
}

/// Get the currently active/foreground application
pub fn get_foreground_application() -> Result<Option<AppInfo>, Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let workspace_class = class!(NSWorkspace);
        let workspace: id = msg_send![workspace_class, sharedWorkspace];

        let frontmost: id = msg_send![workspace, frontmostApplication];
        if frontmost == nil {
            return Ok(None);
        }

        let bundle_id_ns: id = msg_send![frontmost, bundleIdentifier];
        if bundle_id_ns == nil {
            return Ok(None);
        }

        let bundle_id = nsstring_to_string(bundle_id_ns);

        let name_ns: id = msg_send![frontmost, localizedName];
        let app_name = if name_ns != nil {
            nsstring_to_string(name_ns)
        } else {
            bundle_id.clone()
        };

        let pid: i32 = msg_send![frontmost, processIdentifier];
        let icon_base64 = get_app_icon_base64(frontmost);

        Ok(Some(AppInfo {
            bundle_id,
            app_name,
            process_id: pid,
            is_active: true,
            icon_base64,
        }))
    }
}

/// Activate/focus an application by bundle ID
pub fn activate_application(bundle_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[tnnl] activate_application called with bundle_id: {}", bundle_id);

    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // First, get the app name from bundle ID
        let workspace_class = class!(NSWorkspace);
        let workspace: id = msg_send![workspace_class, sharedWorkspace];

        let running_apps: id = msg_send![workspace, runningApplications];
        let count: usize = msg_send![running_apps, count];

        let mut app_name: Option<String> = None;

        // Find the app and get its localized name
        for i in 0..count {
            let app: id = msg_send![running_apps, objectAtIndex: i];
            let app_bundle_id_ns: id = msg_send![app, bundleIdentifier];

            if app_bundle_id_ns != nil {
                let app_bundle_id = nsstring_to_string(app_bundle_id_ns);

                if app_bundle_id == bundle_id {
                    let name_ns: id = msg_send![app, localizedName];
                    if name_ns != nil {
                        app_name = Some(nsstring_to_string(name_ns));
                    }
                    break;
                }
            }
        }

        if let Some(name) = app_name {
            println!("[tnnl] Found app: {} ({}), using AppleScript to activate...", name, bundle_id);

            // Use AppleScript to activate the app - this is more reliable
            let script = format!("tell application \"{}\" to activate", name);
            let script_cstring = std::ffi::CString::new(script).unwrap();
            let script_ns = NSString::alloc(nil);
            let script_ns: id = msg_send![script_ns, initWithUTF8String: script_cstring.as_ptr()];

            let apple_script_class = class!(NSAppleScript);
            let apple_script: id = msg_send![apple_script_class, alloc];
            let apple_script: id = msg_send![apple_script, initWithSource: script_ns];

            let mut error: id = nil;
            let _result: id = msg_send![apple_script, executeAndReturnError: &mut error];

            if error == nil {
                println!("[tnnl] ✓ Successfully activated app via AppleScript: {}", name);
                Ok(())
            } else {
                let error_desc: id = msg_send![error, localizedDescription];
                let error_str = if error_desc != nil {
                    nsstring_to_string(error_desc)
                } else {
                    "Unknown error".to_string()
                };
                println!("[tnnl] ✗ AppleScript error: {}", error_str);
                Err(format!("Failed to activate app: {}", error_str).into())
            }
        } else {
            let err_msg = format!("App not found with bundle_id: {}", bundle_id);
            eprintln!("[tnnl] ✗ {}", err_msg);
            Err(err_msg.into())
        }
    }
}

/// Resize an application's main window using Accessibility API
pub fn resize_app_window(
    bundle_id: &str,
    width: f64,
    height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // This requires the Accessibility API which is more complex
    // For now, we'll implement a placeholder
    // Full implementation would use AXUIElementCreateApplication and AXUIElementSetAttributeValue

    println!(
        "[tnnl] Window resize requested for {}: {}x{}",
        bundle_id, width, height
    );
    println!("[tnnl] Note: Window resizing requires Accessibility permissions");
    println!("[tnnl] This feature will be implemented in a follow-up");

    // TODO: Implement using core-foundation's AX APIs:
    // 1. Get app PID from bundle_id
    // 2. Create AXUIElement for application
    // 3. Get main window (AXMainWindow attribute)
    // 4. Set AXSize attribute to new dimensions

    Ok(())
}

// Helper functions

/// Convert NSString to Rust String
unsafe fn nsstring_to_string(ns_string: id) -> String {
    let c_str: *const i8 = msg_send![ns_string, UTF8String];
    std::ffi::CStr::from_ptr(c_str)
        .to_string_lossy()
        .into_owned()
}

/// Get app icon as base64-encoded PNG
unsafe fn get_app_icon_base64(app: id) -> Option<String> {
    let icon: id = msg_send![app, icon];
    if icon == nil {
        return None;
    }

    // Convert NSImage to PNG data
    // Get TIFF representation first
    let tiff_data: id = msg_send![icon, TIFFRepresentation];
    if tiff_data == nil {
        return None;
    }

    // Create NSBitmapImageRep from TIFF
    let bitmap_class = class!(NSBitmapImageRep);
    let bitmap: id = msg_send![bitmap_class, imageRepWithData: tiff_data];
    if bitmap == nil {
        return None;
    }

    // Convert to PNG
    let png_type = 4u64; // NSBitmapImageFileTypePNG
    let properties = nil;
    let png_data: id = msg_send![bitmap, representationUsingType:png_type properties:properties];

    if png_data == nil {
        return None;
    }

    // Get bytes from NSData
    let length: usize = msg_send![png_data, length];
    let bytes: *const u8 = msg_send![png_data, bytes];

    if bytes.is_null() || length == 0 {
        return None;
    }

    // Convert to base64
    let slice = std::slice::from_raw_parts(bytes, length);
    Some(base64_encode(slice))
}

/// Base64 encode bytes
fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder = base64::write::EncoderWriter::new(&mut buf, &base64::engine::general_purpose::STANDARD);
        encoder.write_all(data).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

/// Check if we should skip this app from the list
fn should_skip_app(bundle_id: &str) -> bool {
    // Skip system processes and background apps
    let skip_list = [
        "com.apple.loginwindow",
        "com.apple.systemuiserver",
        "com.apple.dock",
        "com.apple.notificationcenterui",
        "com.apple.controlcenter",
        "com.apple.WindowManager",
        "com.apple.Spotlight",
    ];

    skip_list.iter().any(|&skip| bundle_id == skip)
}

/// Window information with bounds
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub window_id: CGWindowID,
    pub owner_pid: i32,
    pub bounds: (f64, f64, f64, f64), // (x, y, width, height)
}

// FFI declarations for Core Graphics window list
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
}

// Constants for CGWindowListCopyWindowInfo
const KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1 << 0;
const KCG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;

/// Get all windows for a given process ID with actual bounds from Core Graphics
pub fn get_windows_for_pid(pid: i32) -> Vec<WindowInfo> {
    unsafe {
        // Get all on-screen windows
        let window_list_ref = CGWindowListCopyWindowInfo(
            KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | KCG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            0
        );

        if window_list_ref.is_null() {
            eprintln!("[tnnl] Failed to get window list");
            return vec![];
        }

        let window_list: CFArray<CFDictionary> = CFArray::wrap_under_create_rule(window_list_ref);
        let mut windows = Vec::new();

        // CF dictionary keys
        let key_owner_pid = CFString::from_static_string("kCGWindowOwnerPID");
        let key_window_id = CFString::from_static_string("kCGWindowNumber");
        let key_bounds = CFString::from_static_string("kCGWindowBounds");

        for i in 0..window_list.len() {
            if let Some(window_dict) = window_list.get(i) {
                // Get owner PID
                let window_pid: Option<i32> = window_dict
                    .find(key_owner_pid.as_CFTypeRef())
                    .and_then(|pid_ref| unsafe {
                        let cf_num: CFNumber = CFNumber::wrap_under_get_rule(*pid_ref as *const _);
                        cf_num.to_i32()
                    });

                // Only process windows for the requested PID
                if window_pid != Some(pid) {
                    continue;
                }

                // Get window ID
                let window_id: u32 = window_dict
                    .find(key_window_id.as_CFTypeRef())
                    .and_then(|id_ref| unsafe {
                        let cf_num: CFNumber = CFNumber::wrap_under_get_rule(*id_ref as *const _);
                        cf_num.to_i64().map(|v| v as u32)
                    })
                    .unwrap_or(0);

                // Get bounds dictionary
                let bounds: (f64, f64, f64, f64) = window_dict
                    .find(key_bounds.as_CFTypeRef())
                    .and_then(|bounds_ref| unsafe {
                        let bounds_dict: CFDictionary = CFDictionary::wrap_under_get_rule(*bounds_ref as *const _);

                        let key_x = CFString::from_static_string("X");
                        let key_y = CFString::from_static_string("Y");
                        let key_width = CFString::from_static_string("Width");
                        let key_height = CFString::from_static_string("Height");

                        let x = bounds_dict.find(key_x.as_CFTypeRef())
                            .and_then(|v| {
                                let cf_num: CFNumber = CFNumber::wrap_under_get_rule(*v as *const _);
                                cf_num.to_f64()
                            })?;
                        let y = bounds_dict.find(key_y.as_CFTypeRef())
                            .and_then(|v| {
                                let cf_num: CFNumber = CFNumber::wrap_under_get_rule(*v as *const _);
                                cf_num.to_f64()
                            })?;
                        let width = bounds_dict.find(key_width.as_CFTypeRef())
                            .and_then(|v| {
                                let cf_num: CFNumber = CFNumber::wrap_under_get_rule(*v as *const _);
                                cf_num.to_f64()
                            })?;
                        let height = bounds_dict.find(key_height.as_CFTypeRef())
                            .and_then(|v| {
                                let cf_num: CFNumber = CFNumber::wrap_under_get_rule(*v as *const _);
                                cf_num.to_f64()
                            })?;

                        Some((x, y, width, height))
                    })
                    .unwrap_or((0.0, 0.0, 0.0, 0.0));

                // Skip windows with invalid bounds
                if bounds.2 > 50.0 && bounds.3 > 50.0 {
                    windows.push(WindowInfo {
                        window_id,
                        owner_pid: pid,
                        bounds,
                    });
                }
            }
        }

        println!("[tnnl] Found {} windows for PID {}", windows.len(), pid);
        windows
    }
}

/// Get the main window for the foreground application
/// Returns window ID and bounds: (window_id, x, y, width, height)
pub fn get_frontmost_window() -> Option<(CGWindowID, f64, f64, f64, f64)> {
    // Get foreground app PID
    let app_info = get_foreground_application().ok()??;
    let pid = app_info.process_id;

    // Get windows for this PID
    let windows = get_windows_for_pid(pid);

    // Return the first window (CGWindowListCopyWindowInfo returns in front-to-back order)
    // This ensures we get the actually focused window, not just the largest one
    windows.into_iter()
        .next()
        .map(|w| (w.window_id, w.bounds.0, w.bounds.1, w.bounds.2, w.bounds.3))
}

/// Global state for window focus observer running flag
static FOCUS_OBSERVER_RUNNING: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

/// Start observing window focus changes and update crop automatically
/// This sets up a macOS notification observer that listens for app activation
pub async fn start_focus_observer() -> Result<(), Box<dyn std::error::Error>> {
    // Check if already running
    let mut is_running = FOCUS_OBSERVER_RUNNING.lock().await;
    if *is_running {
        println!("[tnnl] Focus observer already running, skipping");
        return Ok(());
    }
    *is_running = true;
    drop(is_running);

    // For now, we'll use polling every 500ms to detect focus changes
    // A proper implementation would use NSWorkspace notifications with blocks
    tokio::spawn(async move {
        let mut last_app: Option<String> = None;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Check if we should stop
            if !*FOCUS_OBSERVER_RUNNING.lock().await {
                println!("[tnnl] Focus observer stopped");
                break;
            }

            if let Ok(Some(app_info)) = get_foreground_application() {
                let current_app = format!("{}:{}", app_info.bundle_id, app_info.process_id);

                if last_app.as_ref() != Some(&current_app) {
                    println!("[tnnl] Focus changed to: {}", app_info.app_name);
                    last_app = Some(current_app);

                    // Refresh window crop
                    if let Err(e) = crate::screen_capture::refresh_window_crop().await {
                        eprintln!("[tnnl] Failed to refresh crop on focus change: {}", e);
                    } else {
                        println!("[tnnl] ✓ Crop updated for {}", app_info.app_name);
                    }
                }
            }
        }
    });

    println!("[tnnl] Window focus observer started");
    Ok(())
}

/// Stop observing window focus changes
pub async fn stop_focus_observer() -> Result<(), Box<dyn std::error::Error>> {
    let mut is_running = FOCUS_OBSERVER_RUNNING.lock().await;
    *is_running = false;
    drop(is_running);

    println!("[tnnl] Window focus observer stop requested");
    Ok(())
}

