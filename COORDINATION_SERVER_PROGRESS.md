# tnnl.to Coordination Server - Implementation Progress

## ✅ Phase 1: Coordination Server Core (COMPLETED)

### 1.1 Optional Password Support
- ✅ Added `password: Option<String>` to `Tunnel` struct
- ✅ Updated `TunnelManager` methods to accept optional password parameter
- ✅ Modified `NginxManager` to conditionally add `auth_basic` directives
- ✅ Username hardcoded to `tnnl` for simplicity

### 1.2 PostgreSQL Integration
- ✅ Created migration files (`migrations/20250107000001_initial_schema.{up,down}.sql`)
- ✅ Added `users` and `tunnels` tables with proper indexes
- ✅ Created `db.rs` module with database operations
- ✅ Integrated `sqlx::PgPool` into `AppState`
- ✅ Auto-run migrations on server startup

### 1.3 JWT Authentication
- ✅ Implemented `AuthService` in `auth.rs`
- ✅ Verify Supabase JWT tokens (HS256 algorithm)
- ✅ Extract user_id and email from token claims
- ✅ Handle authentication flow in WebSocket messages

### 1.4 Tunnel Request Handling
- ✅ Complete `handle_message` function with:
  - `auth` message: Verify JWT and store user session
  - `request_tunnel` message: Create tunnel, Nginx config, and respond
  - `heartbeat` message: Respond with acknowledgment
- ✅ Error handling with descriptive messages
- ✅ Database persistence for tunnels and users
- ✅ Cleanup on tunnel creation failure

## 📋 Phase 2: Desktop App Integration (TODO)

### 2.1 Coordination Client Module
**File to create:** `src-tauri/src/coordination_client.rs`

Features needed:
- WebSocket client connection to coordination server
- Send auth message with Supabase token
- Send tunnel request message
- Handle `tunnel_assigned` response
- Store tunnel credentials
- Reconnection logic
- Tauri commands for tunnel status

### 2.2 Desktop App UI Updates
**Files to modify:**
- `index.html` (add tunnel info section)
- `src/main.ts` (add tunnel UI logic)

Features needed:
- Display assigned tunnel URL (e.g., `https://fuzzy-cat-1234.tnnl.to`)
- Show connection status to coordination server
- Optional password input field
- "Custom subdomain (Coming Soon)" placeholder
- Port forwarding status

## 🚀 Phase 3: Production Deployment (TODO)

### 3.1 Server Setup
- [ ] SSH into production server
- [ ] Install PostgreSQL and create `tnnl` database
- [ ] Install Nginx
- [ ] Create directories: `/etc/nginx/tunnels/`, `/etc/nginx/passwd/`
- [ ] Set up wildcard SSL certificate for `*.tnnl.to`
- [ ] Configure firewall rules

### 3.2 DNS Configuration
- [ ] Add wildcard A record: `*.tnnl.to` → server IP
- [ ] Verify DNS propagation

### 3.3 Deployment
- [ ] Set up `.env` file with:
  - `DATABASE_URL`
  - `JWT_SECRET` (use Supabase JWT secret)
  - `BIND_ADDRESS`
- [ ] Build coordination server: `cargo build --release`
- [ ] Create systemd service
- [ ] Start and enable service

## 📝 Configuration Required

### Coordination Server `.env`
```bash
BIND_ADDRESS=0.0.0.0:8080
DATABASE_URL=postgresql://user:password@localhost/tnnl
JWT_SECRET=<SUPABASE_JWT_SECRET>
```

### Desktop App
Add coordination server URL to environment or config:
```
COORDINATION_SERVER_URL=wss://coord.tnnl.to:8080
```

## 🔍 Testing Checklist

### Local Testing (Before Deploy)
- [ ] Start PostgreSQL locally
- [ ] Run coordination server locally
- [ ] Test auth flow with valid Supabase token
- [ ] Test tunnel creation
- [ ] Verify Nginx config generation (mock)
- [ ] Test password protected tunnels
- [ ] Test tunnels without passwords

### Production Testing (After Deploy)
- [ ] Desktop app connects to coordination server
- [ ] Tunnel assignment works
- [ ] HTTPS works for assigned subdomain
- [ ] Password auth works (when enabled)
- [ ] Screen streaming through tunnel
- [ ] Tunnel persists across desktop app restarts
- [ ] Cleanup on disconnect

## 📖 API Reference

### WebSocket Messages

**Client → Server:**

```json
// Authenticate
{
  "type": "auth",
  "token": "jwt-token-from-supabase"
}

// Request tunnel
{
  "type": "request_tunnel",
  "password": "optional-password"  // Omit for no password
}

// Heartbeat
{
  "type": "heartbeat"
}
```

**Server → Client:**

```json
// Auth success
{
  "type": "auth_success",
  "user_id": "uuid",
  "email": "user@example.com"
}

// Tunnel assigned
{
  "type": "tunnel_assigned",
  "tunnel": {
    "id": "uuid",
    "subdomain": "fuzzy-cat-1234",
    "url": "https://fuzzy-cat-1234.tnnl.to",
    "port": 10000,
    "password": "optional-password",
    "created_at": "2025-01-07T..."
  }
}

// Heartbeat ack
{
  "type": "heartbeat_ack",
  "timestamp": "2025-01-07T..."
}

// Error
{
  "type": "error",
  "message": "Error description"
}
```

## 🎯 Next Steps

1. **Implement desktop app coordination client** (Phase 2.1)
2. **Update desktop app UI** (Phase 2.2)
3. **Test end-to-end locally**
4. **Deploy to production server** (Phase 3)
5. **Add tunnel cleanup on disconnect** (enhancement)
6. **Add metrics and monitoring** (enhancement)

## 📁 Modified Files Summary

### Coordination Server
- ✅ `backend/coordination-server/src/tunnel.rs` - Added optional password
- ✅ `backend/coordination-server/src/nginx.rs` - Conditional auth directives
- ✅ `backend/coordination-server/src/auth.rs` - JWT verification
- ✅ `backend/coordination-server/src/db.rs` - NEW: Database operations
- ✅ `backend/coordination-server/src/main.rs` - Complete request handling
- ✅ `backend/coordination-server/migrations/` - NEW: SQL migrations

### Desktop App (Pending)
- ⏳ `src-tauri/src/coordination_client.rs` - NEW
- ⏳ `src-tauri/src/lib.rs` - Add coordination module
- ⏳ `src/main.ts` - Add tunnel UI
- ⏳ `index.html` - Add tunnel section
