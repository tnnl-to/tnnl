# tnnl Landing Page - Deployment Notes

## GitHub Repository & Release Access

### ⚠️ Important: Private Repos = Private Releases

GitHub **does not allow public releases on private repositories**. All releases and their downloadable assets are only accessible to users with repository access. This means:
- Landing page GitHub API calls will fail without authentication
- Public users cannot download release binaries
- Downloads won't work on your public website

### Option 1: Make Repository Public ⭐ RECOMMENDED

The best and simplest solution is to make the repository public with an MIT license (included at `/LICENSE`).

**Why this is recommended:**
- GitHub API works without authentication
- Users can download releases directly
- Shows transparency and builds trust
- "Raw" or "early stage" code is fine - many successful projects start this way
- You can always clean up documentation/code before heavy marketing
- Can make private again later if truly needed

**To do this:**
1. Go to your repo Settings
2. Scroll to "Danger Zone"
3. Click "Change repository visibility" → "Make public"
4. The MIT license is already in `/LICENSE`

### Option 2: Use GitHub Token (For Private Repos)

If you want to keep the repo private, you can use a GitHub Personal Access Token:

1. **Create a fine-grained token** at https://github.com/settings/tokens:
   - Go to Settings → Developer settings → Personal access tokens → Fine-grained tokens
   - Click "Generate new token"
   - Set permissions: Repository → Contents (Read-only)
   - Scope it to just the `tnnl-to/tnnl` repository

2. **Deploy token securely:**

   **Option A: Server-side proxy (recommended for production)**
   - Create a simple API endpoint on your coordination server
   - Store the token server-side as an environment variable
   - Proxy GitHub API requests through your server

   ```rust
   // Example endpoint in coordination server:
   // GET /api/releases/latest
   // Returns GitHub release data with token auth on backend
   ```

   Then update `app.js`:
   ```javascript
   const response = await fetch('/api/releases/latest');
   ```

   **Option B: Client-side token (only for testing/staging)**
   - Open browser console on landing page
   - Run: `localStorage.setItem('github_token', 'ghp_yourtoken')`
   - Refresh page - downloads should work
   - ⚠️ **DO NOT** commit tokens to code or deploy this way

3. **Update fetch URL** if needed:
   ```javascript
   // app.js already supports tokens via localStorage
   // Just set the token and it will work
   ```

## Current State

The landing page is configured to:
- ✅ Fetch from `https://api.github.com/repos/tnnl-to/tnnl/releases/latest`
- ✅ Support authentication via localStorage token
- ✅ Gracefully fall back to "Coming Soon" if release not found
- ✅ Automatically enable download buttons when assets are available
- ✅ Track downloads in Umami analytics

## Asset Detection

The page looks for these patterns in release assets:

**macOS:**
- `darwin`, `macos`, `.dmg`, `aarch64-apple`

**Windows:**
- `windows`, `.exe`, `.msi`, `x86_64-pc-windows`

**Linux:**
- `linux`, `.AppImage`, `.deb`, `x86_64-unknown-linux`

Make sure your GitHub Actions build process names assets accordingly.

## Recommendation

**For production:** Make the repo public OR implement server-side proxy.

**For development/testing:** Use the localStorage token method to test downloads.
