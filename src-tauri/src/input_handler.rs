use cocoa::appkit::NSEvent;
use cocoa::base::nil;
use cocoa::foundation::{NSPoint, NSAutoreleasePool};
use core_graphics::display::CGDisplay;
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

/// Check if the app has Accessibility permissions on macOS
#[cfg(target_os = "macos")]
fn check_accessibility_permission() -> bool {
    // Try to create an event source with HID system state
    // This will fail if we don't have Accessibility permissions
    match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[cfg(not(target_os = "macos"))]
fn check_accessibility_permission() -> bool {
    true // Non-macOS platforms don't need this check
}

/// Mouse input control for macOS
pub struct InputController {}

impl InputController {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(InputController {})
    }

    fn get_event_source() -> Result<CGEventSource, Box<dyn std::error::Error>> {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create CGEventSource".into())
    }

    /// Move mouse cursor to absolute screen coordinates
    pub fn move_mouse(&self, x: f64, y: f64) -> Result<(), Box<dyn std::error::Error>> {
        if !check_accessibility_permission() {
            return Err("Accessibility permission required. Please grant permission in System Settings > Privacy & Security > Accessibility".into());
        }

        println!("[tnnl] Moving mouse to ({}, {})", x, y);
        let point = CGPoint::new(x, y);
        let event_source = Self::get_event_source()?;

        let event = CGEvent::new_mouse_event(
            event_source,
            CGEventType::MouseMoved,
            point,
            CGMouseButton::Left,
        ).map_err(|_| "Failed to create mouse move event")?;

        event.post(CGEventTapLocation::HID);
        println!("[tnnl] Mouse moved successfully");
        Ok(())
    }

    /// Perform a mouse click at current position
    pub fn click(&self, button: MouseButton) -> Result<(), Box<dyn std::error::Error>> {
        if !check_accessibility_permission() {
            return Err("Accessibility permission required. Please grant permission in System Settings > Privacy & Security > Accessibility".into());
        }

        println!("[tnnl] Clicking {:?} button", button);
        let location = self.get_mouse_location();
        let event_source = Self::get_event_source()?;

        let cg_button = match button {
            MouseButton::Left => CGMouseButton::Left,
            MouseButton::Right => CGMouseButton::Right,
            MouseButton::Middle => CGMouseButton::Center,
        };

        let (down_type, up_type) = match button {
            MouseButton::Left => (CGEventType::LeftMouseDown, CGEventType::LeftMouseUp),
            MouseButton::Right => (CGEventType::RightMouseDown, CGEventType::RightMouseUp),
            MouseButton::Middle => (CGEventType::OtherMouseDown, CGEventType::OtherMouseUp),
        };

        // Mouse down
        let down_event = CGEvent::new_mouse_event(
            event_source.clone(),
            down_type,
            location,
            cg_button,
        ).map_err(|_| "Failed to create mouse down event")?;
        down_event.post(CGEventTapLocation::HID);

        // Mouse up
        let up_event = CGEvent::new_mouse_event(
            event_source,
            up_type,
            location,
            cg_button,
        ).map_err(|_| "Failed to create mouse up event")?;
        up_event.post(CGEventTapLocation::HID);

        println!("[tnnl] Click completed successfully");
        Ok(())
    }

    /// Perform a drag operation
    pub fn drag(&self, start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> Result<(), Box<dyn std::error::Error>> {
        let start_point = CGPoint::new(start_x, start_y);
        let end_point = CGPoint::new(end_x, end_y);
        let event_source = Self::get_event_source()?;

        // Mouse down at start
        let down_event = CGEvent::new_mouse_event(
            event_source.clone(),
            CGEventType::LeftMouseDown,
            start_point,
            CGMouseButton::Left,
        ).map_err(|_| "Failed to create drag start event")?;
        down_event.post(CGEventTapLocation::HID);

        // Drag to end
        let drag_event = CGEvent::new_mouse_event(
            event_source.clone(),
            CGEventType::LeftMouseDragged,
            end_point,
            CGMouseButton::Left,
        ).map_err(|_| "Failed to create drag event")?;
        drag_event.post(CGEventTapLocation::HID);

        // Mouse up at end
        let up_event = CGEvent::new_mouse_event(
            event_source,
            CGEventType::LeftMouseUp,
            end_point,
            CGMouseButton::Left,
        ).map_err(|_| "Failed to create drag end event")?;
        up_event.post(CGEventTapLocation::HID);

        Ok(())
    }

    /// Scroll the mouse wheel
    pub fn scroll(&self, delta_x: i32, delta_y: i32) -> Result<(), Box<dyn std::error::Error>> {
        if !check_accessibility_permission() {
            return Err("Accessibility permission required. Please grant permission in System Settings > Privacy & Security > Accessibility".into());
        }

        println!("[tnnl] Scrolling: delta_x={}, delta_y={}", delta_x, delta_y);
        // Get current mouse location for scroll event
        let location = self.get_mouse_location();
        let event_source = Self::get_event_source()?;

        // Create scroll event using mouse wheel event
        let event = CGEvent::new_mouse_event(
            event_source,
            CGEventType::ScrollWheel,
            location,
            CGMouseButton::Left,
        ).map_err(|_| "Failed to create scroll event")?;

        // Set scroll deltas
        event.set_integer_value_field(core_graphics::event::EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1, delta_y as i64);
        event.set_integer_value_field(core_graphics::event::EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_2, delta_x as i64);

        event.post(CGEventTapLocation::HID);
        println!("[tnnl] Scroll completed successfully");
        Ok(())
    }

    /// Get current mouse location
    fn get_mouse_location(&self) -> CGPoint {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let location: NSPoint = NSEvent::mouseLocation(nil);
            CGPoint::new(location.x, location.y)
        }
    }

    /// Get screen dimensions for coordinate mapping
    pub fn get_screen_size() -> (f64, f64) {
        let display = CGDisplay::main();
        let width = display.pixels_wide() as f64;
        let height = display.pixels_high() as f64;
        (width, height)
    }

    /// Send a single key press (no modifiers)
    pub fn send_key(&self, key_code: u16) -> Result<(), Box<dyn std::error::Error>> {
        if !check_accessibility_permission() {
            return Err("Accessibility permission required".into());
        }

        println!("[tnnl] Sending key: {}", key_code);
        let event_source = Self::get_event_source()?;

        // Key down
        let key_down = CGEvent::new_keyboard_event(event_source.clone(), key_code, true)
            .map_err(|_| "Failed to create key down event")?;
        key_down.post(CGEventTapLocation::HID);

        // Key up
        let key_up = CGEvent::new_keyboard_event(event_source, key_code, false)
            .map_err(|_| "Failed to create key up event")?;
        key_up.post(CGEventTapLocation::HID);

        println!("[tnnl] Key sent successfully");
        Ok(())
    }

    /// Send a key combination with modifiers (e.g., Cmd+`)
    pub fn send_key_combination(&self, key_code: u16, cmd: bool, shift: bool, alt: bool, ctrl: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !check_accessibility_permission() {
            return Err("Accessibility permission required".into());
        }

        println!("[tnnl] Sending key combo: key={}, cmd={}, shift={}, alt={}, ctrl={}",
                 key_code, cmd, shift, alt, ctrl);
        let event_source = Self::get_event_source()?;

        // Create key down event
        let key_down = CGEvent::new_keyboard_event(event_source.clone(), key_code, true)
            .map_err(|_| "Failed to create key down event")?;

        // Set modifier flags
        let mut flags: u64 = 0;
        if cmd { flags |= 0x100000; }    // kCGEventFlagMaskCommand
        if shift { flags |= 0x20000; }   // kCGEventFlagMaskShift
        if alt { flags |= 0x80000; }     // kCGEventFlagMaskAlternate
        if ctrl { flags |= 0x40000; }    // kCGEventFlagMaskControl

        key_down.set_flags(core_graphics::event::CGEventFlags::from_bits_truncate(flags));
        key_down.post(CGEventTapLocation::HID);

        // Key up
        let key_up = CGEvent::new_keyboard_event(event_source, key_code, false)
            .map_err(|_| "Failed to create key up event")?;
        key_up.post(CGEventTapLocation::HID);

        println!("[tnnl] Key combination sent successfully");
        Ok(())
    }

    /// Type a string of characters
    pub fn type_string(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !check_accessibility_permission() {
            return Err("Accessibility permission required".into());
        }

        println!("[tnnl] Typing text: {}", text);
        let event_source = Self::get_event_source()?;

        for ch in text.chars() {
            // Create a keyboard event with the Unicode character
            if let Ok(event) = CGEvent::new_keyboard_event(event_source.clone(), 0, true) {
                // Set the Unicode string for this event
                event.set_string_from_utf16_unchecked(&[ch as u16]);
                event.post(CGEventTapLocation::HID);

                // Key up
                if let Ok(up_event) = CGEvent::new_keyboard_event(event_source.clone(), 0, false) {
                    up_event.post(CGEventTapLocation::HID);
                }
            }
        }

        println!("[tnnl] Text typed successfully");
        Ok(())
    }
}

/// Mouse button types
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Global input controller instance
use once_cell::sync::Lazy;
use std::sync::Mutex;

static INPUT_CONTROLLER: Lazy<Mutex<Option<InputController>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the input controller
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let controller = InputController::new()?;
    let mut global = INPUT_CONTROLLER.lock().unwrap();
    *global = Some(controller);
    Ok(())
}

/// Execute an input action using the global controller
pub fn with_controller<F, R>(f: F) -> Result<R, Box<dyn std::error::Error>>
where
    F: FnOnce(&InputController) -> Result<R, Box<dyn std::error::Error>>,
{
    let global = INPUT_CONTROLLER.lock().unwrap();
    match global.as_ref() {
        Some(controller) => f(controller),
        None => Err("Input controller not initialized".into()),
    }
}

/// Coordinate mapping from client screen to Mac screen
pub fn map_coordinates(
    client_x: f64,
    client_y: f64,
    client_width: f64,
    client_height: f64,
) -> (f64, f64) {
    let (screen_width, screen_height) = InputController::get_screen_size();

    let mac_x = (client_x / client_width) * screen_width;
    let mac_y = (client_y / client_height) * screen_height;

    (mac_x, mac_y)
}

/// Check if we have Accessibility permissions (public API)
pub fn has_accessibility_permission() -> bool {
    check_accessibility_permission()
}

/// Request Accessibility permissions by opening System Settings
#[cfg(target_os = "macos")]
pub fn request_accessibility_permission() -> Result<(), Box<dyn std::error::Error>> {
    println!("[tnnl] Opening System Settings to request Accessibility permission...");

    // Open System Settings to Privacy & Security > Accessibility
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility_permission() -> Result<(), Box<dyn std::error::Error>> {
    Ok(()) // No-op on non-macOS platforms
}
