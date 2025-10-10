# Development Roadmap - Next Steps

## Immediate Priority: Screen Capture

### Step 1: Implement ScreenCaptureKit Integration

**File to edit**: `src-tauri/src/screen_capture.rs`

The `screencapturekit` crate is already in dependencies. You need to:

1. Initialize SCStreamConfiguration
2. Get available displays/windows with SCShareableContent
3. Create SCStream with a delegate to receive frames
4. Convert frames to a format suitable for streaming

**Resources**:
- [screencapturekit crate docs](https://docs.rs/screencapturekit/)
- [Apple ScreenCaptureKit docs](https://developer.apple.com/documentation/screencapturekit)
- [WWDC 2022 Video](https://developer.apple.com/videos/play/wwdc2022/10156/)

### Step 2: Frame Buffer Management

Create a circular buffer to hold captured frames:
- Store recent frames in memory
- Manage frame rate (30fps, 60fps options)
- Handle resolution changes
- Implement quality settings

## Phase 2: WebRTC Implementation

### Add Dependencies

```toml
# Add to src-tauri/Cargo.toml
webrtc = "0.11"
tokio-tungstenite = "0.24"  # For signaling
```

### Components Needed

1. **Signaling Server** (separate service or embedded)
   - Can use simple WebSocket server
   - Or use existing service like PeerJS

2. **Peer Connection Setup**
   - STUN/TURN configuration
   - Offer/Answer exchange
   - ICE candidate handling

3. **Media Track**
   - Create video track from captured frames
   - H.264 encoding for efficiency
   - Adaptive bitrate based on connection

## Phase 3: Client Application

### Option A: Progressive Web App (Recommended)

Create `client/` directory with:
```
client/
├── index.html
├── app.ts
├── video-player.ts
├── input-handler.ts
└── connection-manager.ts
```

**Benefits**:
- Works on any device with browser
- No App Store required
- Easy updates

### Option B: Native iOS App

Use SwiftUI with WebRTC SDK:
- Better performance
- Native gestures
- Background connectivity
- Can publish to App Store

## Phase 4: Orchestration Layer

### Window Management

**macOS APIs to integrate**:
- Accessibility API for window control
- CGWindow* functions for window info
- AppleScript bridge for advanced control

**Features to build**:
```rust
// src-tauri/src/window_manager.rs
pub async fn resize_window(window_id: u32, width: u32, height: u32)
pub async fn move_window(window_id: u32, x: i32, y: i32)
pub async fn get_all_windows() -> Vec<WindowInfo>
pub async fn apply_layout_preset(preset_name: &str)
```

### Device Profiles

Create profile system:
```json
{
  "profiles": {
    "iPhone_15_Pro": {
      "width": 1179,
      "height": 2556,
      "safeArea": { "top": 59, "bottom": 34 }
    },
    "iPad_Air": {
      "width": 1640,
      "height": 2360
    }
  }
}
```

### Smart Layouts

Preset layouts like:
- "Terminal Only" - Full screen terminal
- "Code + Browser" - Side by side
- "Stack" - Vertical arrangement
- "Presentation" - Hide distractions

## Testing Strategy

### Local Testing
1. Test screen capture on this Mac
2. Connect from iPhone on same WiFi
3. Test outside local network (TURN server)

### Performance Metrics
- Latency (target: <100ms)
- Frame rate (target: 30fps minimum)
- Bandwidth usage
- Battery impact on client

## Security Considerations

1. **Authentication**
   - Implement token-based auth
   - Consider biometric unlock on client

2. **Encryption**
   - WebRTC handles encryption (DTLS-SRTP)
   - Add TLS for signaling

3. **Authorization**
   - Limit connections to your devices
   - Consider device fingerprinting

## Code Quality

As you build:
- Write tests for core functionality
- Document Rust APIs with `///` comments
- Keep functions small and focused
- Use proper error handling (`Result<T, E>`)

## Useful Commands

```bash
# Run with logging
RUST_LOG=debug npm run tauri dev

# Build optimized
npm run tauri build -- --target universal-apple-darwin

# Check Rust code
cargo clippy --manifest-path=src-tauri/Cargo.toml

# Format code
cargo fmt --manifest-path=src-tauri/Cargo.toml
npm run format  # (add prettier for TS)
```

## Where to Get Help

- Tauri Discord: https://discord.com/invite/tauri
- WebRTC community
- /r/rust subreddit
- Stack Overflow (`[tauri]` `[webrtc]` tags)

## Milestone Checklist

- [ ] Basic screen capture working
- [ ] Local WebRTC connection established
- [ ] Video streaming to web client
- [ ] Mouse input forwarding
- [ ] Keyboard input forwarding
- [ ] Remote WebRTC (through TURN)
- [ ] Mobile client responsive design
- [ ] First orchestration preset
- [ ] Window management working
- [ ] Device profile system
- [ ] Production-ready security
- [ ] App Store submission (if native iOS)
