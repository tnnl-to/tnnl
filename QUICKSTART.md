# Quick Start Guide

## What We've Built

Initial scaffolding for **tnnl** - a personal remote desktop application with intelligent orchestration features. The foundation is in place with:

- ✅ Tauri + Rust backend
- ✅ TypeScript frontend
- ✅ macOS ScreenCaptureKit integration (placeholder)
- ✅ Basic UI with start/stop capture controls
- ✅ Project builds successfully

## Running the App

### Development Mode

```bash
npm run tauri dev
```

This will:
1. Start Vite dev server on `localhost:1420`
2. Compile Rust backend
3. Launch the native macOS app window

### Building for Production

```bash
npm run tauri build
```

Creates optimized production build in `src-tauri/target/release/`

## Current Functionality

The app currently has:
- **UI**: Basic control interface with start/stop buttons
- **Backend**: Rust commands (`start_screen_capture`, `stop_screen_capture`)
- **State Management**: Simple capture state tracking
- **Status Updates**: Real-time status messages

## Next Steps

To build actual functionality, you'll need to implement:

### 1. Real ScreenCaptureKit Integration
Edit `src-tauri/src/screen_capture.rs` to use the `screencapturekit` crate to actually capture the screen.

### 2. WebRTC Streaming
- Add WebRTC dependencies
- Implement peer-to-peer connection
- Set up signaling server
- Stream captured frames to client

### 3. Client App (PWA)
- Create separate web client project
- Implement video player
- Add input handling (mouse/keyboard)
- Test on iPhone/iPad

### 4. Orchestration Features
- macOS Accessibility API integration
- Window management commands
- Device profile system
- Layout presets

## Project Structure

```
tnnl/
├── src/                    # Frontend TypeScript
│   └── main.ts            # Main app logic
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Tauri app setup
│   │   ├── main.rs        # Entry point
│   │   └── screen_capture.rs  # Screen capture logic
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── index.html             # App HTML
└── package.json           # Node dependencies
```

## macOS Permissions

When you first run the app, macOS will prompt for permissions:
- **Screen Recording**: Required for ScreenCaptureKit
- **Accessibility**: Needed for window management (future)

Grant these in System Settings > Privacy & Security.

## Tips

- Use `console.log` in frontend, `println!` in Rust for debugging
- DevTools auto-open in development mode
- Rust changes require recompile (handled automatically in dev mode)
- Frontend changes hot-reload instantly

## Common Issues

**Build fails with "icon not found"**: Icons regenerated automatically, but if issues persist, check `src-tauri/icons/` contains RGBA PNGs.

**Permission denied errors**: Grant screen recording permission in macOS System Settings.

**WebView doesn't load**: Check port 1420 is available.
