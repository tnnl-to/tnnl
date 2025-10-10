#!/bin/bash
# Setup script for SSH server configuration

set -e

echo "========================================="
echo "tnnl SSH Server Setup Script"
echo "========================================="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)"
  exit 1
fi

# Create tnnl user if it doesn't exist
if ! id "tnnl" &>/dev/null; then
    echo "Creating tnnl user..."
    useradd -r -m -d /home/tnnl -s /bin/bash tnnl
    echo "✓ User 'tnnl' created"
else
    echo "✓ User 'tnnl' already exists"
fi

# Create .ssh directory
echo "Setting up SSH directory..."
mkdir -p /home/tnnl/.ssh
chmod 700 /home/tnnl/.ssh
touch /home/tnnl/.ssh/authorized_keys
chmod 600 /home/tnnl/.ssh/authorized_keys
chown -R tnnl:tnnl /home/tnnl/.ssh
echo "✓ SSH directory configured"

# Configure SSH server for port forwarding
echo "Configuring SSH server..."

SSHD_CONFIG="/etc/ssh/sshd_config"

# Backup existing config
cp $SSHD_CONFIG ${SSHD_CONFIG}.backup.$(date +%Y%m%d_%H%M%S)

# Check and add/update required settings
if ! grep -q "^GatewayPorts yes" $SSHD_CONFIG; then
    echo "GatewayPorts yes" >> $SSHD_CONFIG
    echo "✓ Added GatewayPorts yes"
fi

if ! grep -q "^AllowTcpForwarding yes" $SSHD_CONFIG; then
    echo "AllowTcpForwarding yes" >> $SSHD_CONFIG
    echo "✓ Added AllowTcpForwarding yes"
fi

# Restart SSH service
echo "Restarting SSH service..."
systemctl restart sshd || systemctl restart ssh
echo "✓ SSH service restarted"

echo ""
echo "========================================="
echo "Setup Complete!"
echo "========================================="
echo ""
echo "Next steps:"
echo "1. Desktop apps will automatically generate SSH keys on first launch"
echo "2. SSH public keys will be registered via the coordination server API"
echo "3. Tunnels will be established automatically when users connect"
echo ""
echo "Authorized keys file: /home/tnnl/.ssh/authorized_keys"
echo ""
