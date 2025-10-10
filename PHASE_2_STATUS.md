# Phase 2 Status - WebRTC Integration

## Current Status: Foundation Complete ✅

### What We've Built So Far

**Phase 1 (Complete)**:
- ✅ Screen capture working at 10 FPS
- ✅ Real-time statistics display
- ✅ macOS permissions handling
- ✅ Frame counting and performance monitoring

**Phase 2 (In Progress)**:
- ✅ WebRTC command structure (placeholders)
- ✅ API endpoints for WebRTC operations
- ⏳ Full WebRTC peer connection (needs work)
- ⏳ Video encoding
- ⏳ Signaling server
- ⏳ Client PWA

## WebRTC Architecture Plan

```
┌─────────────────────────────────────────────────────────┐
│                    Host Mac (tnnl app)                  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Screen Capture (10 FPS)                         │  │
│  │    ↓                                              │  │
│  │  Video Encoder (H.264) ←─ needs implementation   │  │
│  │    ↓                                              │  │
│  │  WebRTC Peer Connection                          │  │
│  │    - Sends video track                            │  │
│  │    - Receives control events                      │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────┬────────────────────────────────────────┘
                 │
                 │  WebRTC P2P
                 │  (STUN/TURN for NAT traversal)
                 │
┌────────────────▼────────────────────────────────────────┐
│              Client (Phone/Tablet/Browser)              │
│  ┌──────────────────────────────────────────────────┐  │
│  │  PWA / Web Browser                                │  │
│  │    - Video player                                 │  │
│  │    - Touch/mouse input handler                    │  │
│  │    - Connection UI                                │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Challenges Encountered

### WebRTC Crate API Complexity
The `webrtc` Rust crate (v0.11) has a complex API that's changed significantly in newer versions (0.13). Issues:
- Private API types (`Registry`)
- Async handling complexity with Tauri
- Requires deep understanding of WebRTC internals

### Recommended Approaches

**Option 1: Simplified WebSocket Streaming (Easier)**
Instead of full WebRTC, use a simpler approach:
1. Capture frames → Encode to JPEG/PNG
2. Send via WebSocket to client
3. Client displays images (like VNC)
4. Lower quality but much simpler

**Option 2: Browser-based WebRTC (Medium)**
1. Host serves a local web page with WebRTC sender
2. Client connects via WebRTC directly
3. Use JavaScript WebRTC APIs (more mature)
4. Tauri app acts as signaling server

**Option 3: Full Rust WebRTC (Complex)**
1. Deep dive into `webrtc-rs` crate
2. Implement proper peer connection
3. Handle ICE, STUN, TURN
4. Video encoding pipeline
5. Most powerful but requires significant effort

## Implemented Commands

The following Tauri commands are ready (placeholder implementations):

- `init_webrtc()` - Initialize WebRTC peer connection
- `create_webrtc_offer()` - Generate SDP offer for client
- `set_webrtc_answer(answer)` - Accept client's SDP answer
- `get_webrtc_state()` - Check connection status
- `close_webrtc()` - Close peer connection

## Next Steps

### Immediate (Option 1 - WebSocket):
1. Create WebSocket server in Rust
2. Add JPEG/PNG encoding for frames
3. Build simple web client with `<img>` tag that updates
4. Test locally, then over network

### Near-term (Option 2 - Browser WebRTC):
1. Create HTML page with WebRTC sender
2. Serve via Tauri's built-in HTTP server
3. Use JavaScript getUserMedia for screen
4. Client PWA connects and views

### Long-term (Option 3 - Full WebRTC):
1. Study webrtc-rs examples deeply
2. Update to webrtc crate 0.13
3. Implement proper peer connection
4. Add H.264 encoding
5. Deploy STUN/TURN servers

## Files Modified

- `src-tauri/Cargo.toml` - Added tokio-tungstenite (WebSocket support)
- `src-tauri/src/webrtc_peer.rs` - WebRTC module (placeholder)
- `src-tauri/src/lib.rs` - Added 5 WebRTC commands

## Performance Considerations

Current screen capture:
- 10 FPS @ full resolution (~3024x1964)
- ~36MB per frame (uncompressed RGBA)
- Total: ~360MB/sec data rate

For streaming, we need:
- Video encoding (H.264): ~2-5 Mbps target
- Resolution scaling: maybe 1920x1080 or 1280x720
- Frame rate: 30 FPS for smooth experience
- **Compression ratio needed: ~5000:1**

This is why encoding is critical!

## Recommendation

**I suggest we start with Option 1 (WebSocket + JPEG)** for these reasons:
1. You can test it working end-to-end quickly
2. Simple enough to understand completely
3. Works great on local network
4. Can upgrade to WebRTC later if needed
5. Perfect for your use case (home network access)

Would you like to proceed with the WebSocket approach?

---

**Current Build Status**: ✅ Compiles successfully
**Screen Capture**: ✅ Working (10 FPS)
**WebRTC**: ⏳ Placeholder ready for implementation
