# tnnl Coordination Server

WebSocket-based coordination server for managing tunnel connections between desktop apps and the public *.tnnl.to domain.

## Architecture

The coordination server handles:
- **Persistent WebSocket connections** from desktop app clients
- **Tunnel management** (random/custom subdomain assignment)
- **Dynamic Nginx configuration** for per-tunnel reverse proxy
- **HTTP Basic Authentication** via .htpasswd files
- **Authentication** using JWT tokens (to be implemented)

## How It Works

```
Browser → Nginx (*.tnnl.to) → HTTP Basic Auth →
Coordination Server → WebSocket → Desktop App → Screen Capture
```

1. Desktop app connects to coordination server via WebSocket
2. Server assigns tunnel subdomain (random for free, custom for paid)
3. Server generates Nginx config and .htpasswd file for the tunnel
4. Server reloads Nginx to activate the tunnel
5. Incoming HTTPS requests to `subdomain.tnnl.to` are proxied to the desktop app
6. Desktop app streams screen capture via WebSocket through the tunnel

## Setup

### Prerequisites
- Rust 1.77.2 or later
- PostgreSQL database
- Nginx with SSL configured
- htpasswd command-line tool (from apache2-utils)
- sudo privileges for Nginx config management

### Installation

1. Copy environment template:
```bash
cp .env.example .env
```

2. Edit `.env` with your configuration

3. Create required directories:
```bash
sudo mkdir -p /etc/nginx/tunnels
sudo mkdir -p /etc/nginx/passwd
```

4. Add include directive to main Nginx config (`/etc/nginx/nginx.conf`):
```nginx
http {
    # ...existing config...

    include /etc/nginx/tunnels/*.conf;
}
```

5. Build and run:
```bash
cargo build --release
cargo run --release
```

## Development

Run in development mode:
```bash
cargo run
```

Run with logging:
```bash
RUST_LOG=debug cargo run
```

## Database Schema

See `../migrations/` for database schema (to be created).

## API / WebSocket Messages

### Client → Server

**Authenticate:**
```json
{
  "type": "auth",
  "token": "jwt-token-here"
}
```

**Request Tunnel:**
```json
{
  "type": "request_tunnel",
  "custom_subdomain": "myname"  // Optional, omit for random
}
```

**Heartbeat:**
```json
{
  "type": "heartbeat"
}
```

### Server → Client

**Tunnel Assigned:**
```json
{
  "type": "tunnel_assigned",
  "tunnel": {
    "id": "uuid",
    "subdomain": "fuzzy-cat-1234",
    "url": "https://fuzzy-cat-1234.tnnl.to",
    "password": "generated-password",
    "created_at": "2025-01-06T..."
  }
}
```

**Heartbeat Acknowledgment:**
```json
{
  "type": "heartbeat_ack",
  "timestamp": "2025-01-06T..."
}
```

**Error:**
```json
{
  "type": "error",
  "message": "Error description"
}
```

## Tunnel Naming

- **Free tier**: Random subdomain (e.g., `fuzzy-cat-1234.tnnl.to`)
- **Paid tier**: Custom subdomain (e.g., `myname.tnnl.to`)

Subdomain rules:
- 3-63 characters
- Lowercase alphanumeric and hyphens only
- Cannot start or end with hyphen

## Security

- All tunnels require HTTP Basic Authentication (username: `user`, password: auto-generated)
- Passwords stored as bcrypt hashes in PostgreSQL
- JWT tokens for desktop app authentication
- TLS/SSL termination at Nginx layer

## Next Steps

- [ ] Implement JWT authentication
- [ ] Add PostgreSQL database integration
- [ ] Implement actual tunnel forwarding logic
- [ ] Add Stripe payment webhooks
- [ ] Implement tunnel cleanup on client disconnect
- [ ] Add monitoring and metrics

## License

Proprietary
