#!/bin/bash
set -e

echo "=== tnnl Production Deployment Script ==="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (use sudo)"
    exit 1
fi

# 1. Install dependencies
echo "[1/8] Installing dependencies..."
apt-get update
apt-get install -y nginx certbot python3-certbot-nginx apache2-utils sqlite3

# 2. Create tnnl user
echo "[2/8] Creating tnnl system user..."
if ! id -u tnnl > /dev/null 2>&1; then
    useradd --system --home /opt/tnnl --shell /bin/false tnnl
fi

# 3. Create directories
echo "[3/8] Creating directories..."
mkdir -p /opt/tnnl
mkdir -p /var/lib/tnnl
mkdir -p /etc/nginx/tunnels
mkdir -p /etc/nginx/passwd
chown tnnl:tnnl /opt/tnnl
chown tnnl:tnnl /var/lib/tnnl
chown www-data:www-data /etc/nginx/tunnels
chown www-data:www-data /etc/nginx/passwd

# 4. Copy binary and set permissions
echo "[4/8] Installing coordination server binary..."
if [ ! -f "/root/coordination-server" ]; then
    echo "Error: coordination-server binary not found in /root/"
    echo "Please upload the binary first"
    exit 1
fi
cp /root/coordination-server /opt/tnnl/
chmod +x /opt/tnnl/coordination-server
chown tnnl:tnnl /opt/tnnl/coordination-server

# 5. Install systemd service
echo "[5/8] Installing systemd service..."
cat > /etc/systemd/system/tnnl-coordination.service << 'EOF'
[Unit]
Description=tnnl Coordination Server
After=network.target

[Service]
Type=simple
User=tnnl
WorkingDirectory=/opt/tnnl
ExecStart=/opt/tnnl/coordination-server
Restart=always
RestartSec=5

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=tnnl-coordination

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/tnnl

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable tnnl-coordination.service

# 6. Configure Nginx
echo "[6/8] Configuring Nginx..."
cp /root/nginx-tnnl.conf /etc/nginx/sites-available/tnnl
ln -sf /etc/nginx/sites-available/tnnl /etc/nginx/sites-enabled/tnnl
rm -f /etc/nginx/sites-enabled/default

# Test nginx config
nginx -t

# 7. Set up SSL with Let's Encrypt
echo "[7/8] Setting up SSL certificates..."
echo "IMPORTANT: Make sure tnnl.to and *.tnnl.to DNS records point to this server!"
echo ""
read -p "Press Enter when DNS is configured..."

certbot --nginx -d tnnl.to -d "*.tnnl.to" --agree-tos --non-interactive --email admin@tnnl.to || {
    echo "Certbot failed. You may need to configure DNS manually."
    echo "Run: certbot --nginx -d tnnl.to -d '*.tnnl.to'"
}

# 8. Start services
echo "[8/8] Starting services..."
systemctl restart nginx
systemctl start tnnl-coordination.service

echo ""
echo "=== Deployment Complete! ==="
echo ""
echo "Service status:"
systemctl status tnnl-coordination.service --no-pager
echo ""
echo "Check logs with: journalctl -u tnnl-coordination -f"
echo "Test connection: wss://tnnl.to"
