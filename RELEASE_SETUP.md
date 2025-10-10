# Release Setup Guide

This guide will help you set up automated builds and releases for tnnl.

## ✅ What's Already Done

- GitHub Actions workflow created (`.github/workflows/release.yml`)
- Auto-updater configured in `tauri.conf.json`
- Signing keys generated at `~/.tauri/tnnl.key`

## 📝 Setup Steps

### 1. Add GitHub Secrets

Go to your GitHub repository → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**

Add the following secret:

**Name:** `TAURI_PRIVATE_KEY`
**Value:**
```
dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5RExERS9UcXpVOTA1cHFGMTRrTkt4cG5PWHRBZmJmbjFyM28xcWNONmpwVUFBQkFBQUFBQUFBQUFBQUlBQUFBQThMaUxySFRsM2JjRld4dUJKWlRnRjY5dWNIbFByVitObjc1Ylo1NjBIYUhzUmlJQlIycjRFa0xGQnk5ZGdaZFo0bXZVNEYwWlBJY2VzbFRwMk5QdlcydExOdWZ4U2h3eXhVdkJ6N0ZEWExQYk8ySHFTWlFYOWhvWEFrRUlFMkdGTGx1eTdkaldDa2s9Cg==
```

### 2. (Optional) Add Apple Code Signing

Once you have your Apple Developer account, add these secrets:

- `APPLE_CERTIFICATE` - Your .p12 certificate as base64
- `APPLE_CERTIFICATE_PASSWORD` - Certificate password
- `APPLE_SIGNING_IDENTITY` - Your signing identity (e.g., "Developer ID Application: Your Name")
- `APPLE_ID` - Your Apple ID email
- `APPLE_PASSWORD` - App-specific password from appleid.apple.com
- `APPLE_TEAM_ID` - Your team ID from developer.apple.com

**To get your certificate as base64:**
```bash
base64 -i YourCertificate.p12 | pbcopy
```

### 3. Push to GitHub

```bash
git add .
git commit -m "Add GitHub Actions release workflow"
git push
```

## 🚀 Creating a Release

### Option 1: Tag-based Release (Recommended)

```bash
git tag v0.1.0
git push origin v0.1.0
```

This will automatically:
- Build for macOS (Intel + Apple Silicon)
- Build for Windows
- Build for Linux
- Create a GitHub Release with all installers
- Generate update manifests for auto-updates

### Option 2: Manual Trigger

Go to **Actions** → **Release** → **Run workflow**

## 📦 What Gets Built

- **macOS**: `.dmg` (Intel + Apple Silicon)
- **Windows**: `.msi` and `.exe`
- **Linux**: `.deb`, `.AppImage`

All installers will be attached to the GitHub Release.

## 🔄 Auto-Updates

Users running your app will automatically get notified when a new version is available. The updater is configured to check:
```
https://github.com/kaarch/tnnl/releases/latest/download/latest.json
```

## 🔐 Security Notes

- **NEVER commit** `~/.tauri/tnnl.key` (private key) to Git
- Keep `TAURI_PRIVATE_KEY` secret in GitHub Secrets only
- The public key in `tauri.conf.json` is safe to commit

## 📱 Next Steps for Mobile

Once you have your Apple Developer account:

1. Enroll in the Apple Developer Program ($99/year)
2. Create App ID and provisioning profiles
3. Add the signing secrets to GitHub
4. Re-run the release workflow

Without signing, macOS users will need to right-click → "Open" on first launch.
