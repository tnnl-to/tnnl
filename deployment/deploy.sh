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
mkdir -p /var/www/tnnl/landing
chown tnnl:tnnl /opt/tnnl
chown tnnl:tnnl /var/lib/tnnl
chown www-data:www-data /etc/nginx/tunnels
chown www-data:www-data /etc/nginx/passwd
chown -R www-data:www-data /var/www/tnnl

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

# Copy landing page files
echo "[4.5/8] Installing landing page..."
if [ -d "/root/landing" ]; then
    cp -r /root/landing/* /var/www/tnnl/landing/
    chown -R www-data:www-data /var/www/tnnl/landing
    echo "Landing page installed successfully"
else
    echo "Warning: /root/landing directory not found. Skipping landing page installation."
    echo "Please upload landing page files to /root/landing/"
fi

# Copy client.html template
echo "[4.6/8] Installing client.html template..."
if [ -f "/root/client.html" ]; then
    cp /root/client.html /opt/tnnl/
    chown tnnl:tnnl /opt/tnnl/client.html
    echo "Client template installed successfully"
else
    echo "Warning: /root/client.html not found. Tunnel subdomains will not have client interface."
    echo "Please upload client.html to /root/"
fi

# Create /var/www/html if it doesn't exist
mkdir -p /var/www/html
chown -R www-data:www-data /var/www/html

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

# Add WebSocket upgrade map to main nginx.conf if not already present
if ! grep -q "map \$http_upgrade \$connection_upgrade" /etc/nginx/nginx.conf; then
    echo "Adding WebSocket upgrade map to nginx.conf..."
    # Insert map directive in http block (before the last closing brace)
    sed -i '/^http {/a\    # WebSocket upgrade support for tunnel subdomains\n    map $http_upgrade $connection_upgrade {\n        default upgrade;\n        '"'"''"'"' close;\n    }\n' /etc/nginx/nginx.conf
fi

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
