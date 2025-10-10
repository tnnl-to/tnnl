#!/bin/bash

# Script to bump version across all required files
# Usage: ./bump-version.sh 0.1.3

if [ -z "$1" ]; then
    echo "Error: Version number required"
    echo "Usage: ./bump-version.sh <version>"
    echo "Example: ./bump-version.sh 0.1.3"
    exit 1
fi

VERSION=$1

# Validate version format (basic semver check)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semver format (e.g., 0.1.3)"
    exit 1
fi

echo "Bumping version to $VERSION..."

# Update package.json
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" package.json
echo "✓ Updated package.json"

# Update tauri.conf.json
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
echo "✓ Updated src-tauri/tauri.conf.json"

# Update Cargo.toml
sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
echo "✓ Updated src-tauri/Cargo.toml"

echo ""
echo "Version bump complete! Changed files:"
git status --short package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
