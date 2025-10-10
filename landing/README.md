# tnnl Landing Page

Static landing page for tnnl.to with Umami analytics integration.

## Local Testing

```bash
# Start a simple HTTP server
cd landing
python3 -m http.server 8000

# Open in browser
open http://localhost:8000
```

## Running Tests

```bash
# Run unit tests for landing page functionality
cd landing
node app.test.js
```

Tests cover:
- GitHub API integration and asset detection
- Platform-specific asset patterns (macOS, Windows, Linux)
- localStorage token handling
- Analytics event tracking

## Umami Analytics Setup

Before deploying, update `index.html` with your Umami credentials:

```html
<script defer src="YOUR_UMAMI_SCRIPT_URL" data-website-id="YOUR_WEBSITE_ID"></script>
```

Replace:
- `YOUR_UMAMI_SCRIPT_URL`: Your Umami instance URL (e.g., `https://analytics.yourdomain.com/script.js`)
- `YOUR_WEBSITE_ID`: Your website ID from Umami dashboard

## Custom Events Tracked

The landing page automatically tracks the following events:

- **button_click**: All button interactions
  - Properties: `button_text`, `section`
- **section_view**: Section visibility (50% threshold)
  - Properties: `section`

You can view these events in your Umami dashboard under "Events".

## Deployment

1. **Upload files to server:**
   ```bash
   scp -r landing/ root@YOUR_DROPLET_IP:~/
   scp deployment/nginx-tnnl.conf root@YOUR_DROPLET_IP:~/
   scp deployment/deploy.sh root@YOUR_DROPLET_IP:~/
   ```

2. **Run deployment script:**
   ```bash
   ssh root@YOUR_DROPLET_IP
   chmod +x deploy.sh
   ./deploy.sh
   ```

3. **Verify:**
   - Landing page: https://tnnl.to
   - WebSocket: wss://tnnl.to/ws
   - API (optional): https://tnnl.to/api

## File Structure

```
landing/
├── index.html          # Main HTML with Umami script
├── style.css           # Dark theme styles with design system
├── app.js              # Interactive features + Umami event tracking
├── app.test.js         # Unit tests
├── screenshot.png      # App screenshot (formatted for GitHub)
├── screenshot-raw.png  # App screenshot (raw, used on landing page)
└── README.md           # This file
```

## Notes

- Analytics only logs to console on localhost for development
- Static assets are cached for 1 year in production
- Landing page uses dark theme by default with light mode support