# tnnl Production Deployment Guide

## Prerequisites

- Digital Ocean droplet (Ubuntu 22.04 LTS)
- Domain `tnnl.to` with DNS configured
- Root SSH access to the server

## Step 1: Build the Coordination Server

On your **local Mac**:

```bash
cd ~/tnnl/backend/coordination-server
cargo build --release
```

The binary will be at: `target/release/coordination-server`

## Step 2: Prepare Deployment Files

Upload to your droplet:

```bash
# From your local machine
scp backend/coordination-server/target/release/coordination-server root@YOUR_DROPLET_IP:~/
scp deployment/nginx-tnnl.conf root@YOUR_DROPLET_IP:~/
scp deployment/deploy.sh root@YOUR_DROPLET_IP:~/
```

## Step 3: Configure DNS

Make sure your DNS has these records pointing to your droplet IP:

```
A     tnnl.to           -> YOUR_DROPLET_IP
A     *.tnnl.to         -> YOUR_DROPLET_IP
```

## Step 4: Run Deployment Script

SSH into your droplet and run:

```bash
ssh root@YOUR_DROPLET_IP
chmod +x deploy.sh
./deploy.sh
```

The script will:
1. Install nginx, certbot, and dependencies
2. Create system user and directories
3. Install the coordination server binary
4. Set up systemd service
5. Configure Nginx with SSL
6. Start everything

## Step 5: Verify Deployment

Check the service is running:

```bash
systemctl status tnnl-coordination
journalctl -u tnnl-coordination -f
```

Test WebSocket connection:

```bash
# Should return 426 Upgrade Required (normal for HTTP on WS endpoint)
curl -I https://tnnl.to
```

## Step 6: Build macOS Desktop App

On your **local Mac**:

```bash
cd ~/tnnl
npm run tauri build
```

The app will be at: `src-tauri/target/release/bundle/macos/tnnl.app`

## Testing the Full Flow

1. Run the built app from `tnnl.app`
2. Sign in with any email + code `123456` (production Supabase coming soon)
3. Click "Connect to tnnl.to"
4. You should get a tunnel like: `https://abc123.tnnl.to`
5. Test accessing the tunnel URL from another device

## Troubleshooting

### Coordination server won't start

```bash
# Check logs
journalctl -u tnnl-coordination -n 50

# Check if port 8080 is in use
lsof -i :8080

# Manually test the binary
sudo -u tnnl /opt/tnnl/coordination-server
```

### SSL certificate issues

```bash
# Manually get certificates
certbot --nginx -d tnnl.to -d "*.tnnl.to"

# Check certificate
certbot certificates
```

### Nginx errors

```bash
# Test config
nginx -t

# Check nginx logs
tail -f /var/log/nginx/error.log

# Restart nginx
systemctl restart nginx
```

### Desktop app won't connect

1. Check if coordination server is running: `systemctl status tnnl-coordination`
2. Test WebSocket: `wscat -c wss://tnnl.to` (install wscat: `npm install -g wscat`)
3. Check browser console for errors
4. Verify Supabase token is valid

## Maintenance

### View logs

```bash
# Coordination server logs
journalctl -u tnnl-coordination -f

# Nginx access logs
tail -f /var/log/nginx/access.log

# Nginx error logs
tail -f /var/log/nginx/error.log
```

### Restart services

```bash
systemctl restart tnnl-coordination
systemctl restart nginx
```

### Update coordination server

```bash
# On local Mac, rebuild
cd backend/coordination-server
cargo build --release

# Upload new binary
scp target/release/coordination-server root@YOUR_DROPLET_IP:~/

# On droplet
systemctl stop tnnl-coordination
cp ~/coordination-server /opt/tnnl/
chown tnnl:tnnl /opt/tnnl/coordination-server
chmod +x /opt/tnnl/coordination-server
systemctl start tnnl-coordination
```

### Renew SSL certificates

Certbot auto-renews, but you can manually test:

```bash
certbot renew --dry-run
```

## Production Checklist

- [ ] Coordination server running and accessible at wss://tnnl.to
- [ ] SSL certificates installed and valid
- [ ] Nginx configured and running
- [ ] Desktop app connects successfully
- [ ] Tunnel assignment works
- [ ] Can access tunnel URL externally
- [ ] Password protection works (if set)
- [ ] Logs are clean (no errors)
- [ ] Auto-restart on failure works
- [ ] Monitoring set up (optional: DigitalOcean monitoring)

## Security Notes

- The coordination server runs as a non-root `tnnl` user
- Systemd security hardening is enabled
- Nginx handles SSL termination
- Firewall rules should allow ports: 22 (SSH), 80 (HTTP), 443 (HTTPS)
- Database is SQLite in `/var/lib/tnnl/` (owned by `tnnl` user)

## Next Steps

1. Set up production Supabase project
2. Update environment variables
3. Set up monitoring/alerts
4. Configure backups for `/var/lib/tnnl/`
5. Add rate limiting in Nginx (if needed)
