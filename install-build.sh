#!/bin/bash
# Quick install script for tnnl production builds

echo "Killing any running tnnl instances..."
pkill -9 tnnl 2>/dev/null

echo "Removing old version..."
rm -rf /Applications/tnnl.app

echo "Installing new build..."
cp -R src-tauri/target/release/bundle/macos/tnnl.app /Applications/

echo "✓ Done! Launch tnnl from Applications."
