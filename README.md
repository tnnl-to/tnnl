# tnnl - Personal Remote Desktop with Orchestration

A modern remote desktop application inspired by "Back to My Mac", built with Tauri, Rust, and TypeScript. Designed for secure, low-latency remote access with intelligent window orchestration for different screen sizes.

## Features (Planned)

- **Secure Tunneling**: WebRTC peer-to-peer connections with end-to-end encryption
- **Screen Capture**: Native macOS ScreenCaptureKit integration for high-performance capture
- **Smart Orchestration**: Automatic window resizing and layout based on client device
- **Cross-Platform Client**: Progressive Web App works on iPhone, iPad, and any modern browser
- **Low Latency**: Direct peer connections when possible, minimal relay usage

## Current Status

This is the initial scaffolding. Core functionality includes:
- ✅ Tauri project structure
- ✅ Basic frontend UI
- ✅ Rust backend with ScreenCaptureKit placeholders
- ⏳ Actual screen capture implementation (TODO)
- ⏳ WebRTC peer connection (TODO)
- ⏳ Input handling (TODO)
- ⏳ Orchestration layer (TODO)

## Prerequisites

- macOS 10.15+ (for development)
- Node.js 18+
- Rust 1.77.2+
- Xcode Command Line Tools

## Setup

1. Clone the repository:
```bash
cd tnnl
```

2. Install dependencies:
```bash
npm install
```

3. Run in development mode:
```bash
npm run tauri dev
```

## Development Roadmap

### Phase 1: Foundation (Current)
- [x] Project setup
- [ ] Implement ScreenCaptureKit integration
- [ ] Basic video streaming pipeline
- [ ] Simple connection management

### Phase 2: Connectivity
- [ ] WebRTC peer-to-peer setup
- [ ] Signaling server
- [ ] NAT traversal/STUN/TURN
- [ ] Connection state management

### Phase 3: Interaction
- [ ] Mouse/touch input forwarding
- [ ] Keyboard input handling
- [ ] Clipboard sync
- [ ] File transfer

### Phase 4: Orchestration
- [ ] Screen size detection
- [ ] Window management API integration
- [ ] Device profiles (iPhone, iPad, etc.)
- [ ] Smart layout presets
- [ ] Keyboard shortcuts

### Phase 5: Client Apps
- [ ] Progressive Web App
- [ ] iOS-optimized UI
- [ ] Connection bookmarks
- [ ] Quality settings

## Architecture

```
┌─────────────────────────────────────────┐
│         Client (PWA/Browser)            │
│  ┌─────────────────────────────────┐   │
│  │   UI (React/Svelte)             │   │
│  │   - Connection Manager          │   │
│  │   - Video Player                │   │
│  │   - Input Handler               │   │
│  └─────────────────────────────────┘   │
│            │                            │
│            │ WebRTC                     │
│            │                            │
└────────────┼────────────────────────────┘
             │
┌────────────┼────────────────────────────┐
│            │                            │
│  ┌─────────▼─────────────────────────┐ │
│  │   Host App (Tauri + Rust)        │ │
│  │                                   │ │
│  │   - ScreenCaptureKit             │ │
│  │   - WebRTC Peer                  │ │
│  │   - Window Manager               │ │
│  │   - Input Synthesizer            │ │
│  └───────────────────────────────────┘ │
│         macOS Host Machine             │
└─────────────────────────────────────────┘
```

## License

MIT
