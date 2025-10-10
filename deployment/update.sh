#!/bin/bash
set -e

DROPLET_IP="134.199.229.33"
DROPLET_USER="root"

echo "=== tnnl Backend Update Script ==="
echo ""

# 1. Build coordination server locally
echo "[1/4] Building coordination server..."
cd backend/coordination-server
# Use debug build for now (allows insecure token validation for testing)
cargo build
cd ../..

# 2. Upload binary to droplet
echo "[2/4] Uploading binary to droplet..."
scp backend/coordination-server/target/debug/tnnl-coordination-server ${DROPLET_USER}@${DROPLET_IP}:~/coordination-server

# 3. Deploy on droplet
echo "[3/4] Deploying on droplet..."
ssh ${DROPLET_USER}@${DROPLET_IP} << 'ENDSSH'
systemctl stop tnnl-coordination
cp ~/coordination-server /opt/tnnl/
chown tnnl:tnnl /opt/tnnl/coordination-server
chmod +x /opt/tnnl/coordination-server
systemctl start tnnl-coordination
echo "Waiting for service to start..."
sleep 2
systemctl status tnnl-coordination --no-pager
ENDSSH

# 4. Verify
echo "[4/4] Verifying deployment..."
echo ""
echo "=== Deployment Complete! ==="
echo ""
echo "Server logs:"
ssh ${DROPLET_USER}@${DROPLET_IP} "journalctl -u tnnl-coordination -n 20 --no-pager"
echo ""
echo "Test connection: wss://tnnl.to"
