// Nginx configuration management
use std::path::Path;
use std::process::Command;

use crate::tunnel::Tunnel;

const NGINX_CONF_DIR: &str = "/etc/nginx/tunnels";
const NGINX_PASSWD_DIR: &str = "/etc/nginx/passwd";

pub struct NginxManager {
    // Configuration paths
}

impl NginxManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate Nginx server block for a tunnel
    pub async fn create_tunnel_config(&self, tunnel: &Tunnel) -> anyhow::Result<()> {
        let subdomain = &tunnel.subdomain;
        let port = tunnel.port;

        println!("[Nginx] Creating configuration for tunnel: {}", subdomain);

        // Build optional auth_basic directives
        let auth_config = if let Some(_password) = &tunnel.password {
            format!(
                r#"
    auth_basic "Tunnel Access";
    auth_basic_user_file {passwd_dir}/{subdomain}.htpasswd;
"#,
                passwd_dir = NGINX_PASSWD_DIR,
                subdomain = subdomain
            )
        } else {
            String::new()
        };

        // Generate server block config with HTTP + HTTPS
        // Serves HTML for browser, proxies WebSocket for WS connections
        let config = format!(
            r#"map $http_upgrade $connection_upgrade {{
    default upgrade;
    '' close;
}}

server {{
    listen 80;
    listen [::]:80;
    server_name {subdomain}.tnnl.to;

    # ACME challenge location for certbot
    location /.well-known/acme-challenge/ {{
        root /var/www/certbot;
    }}

    # Redirect all other traffic to HTTPS
    location / {{
        return 301 https://$server_name$request_uri;
    }}
}}

server {{
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name {subdomain}.tnnl.to;

    # SSL certificates (will be created by certbot)
    ssl_certificate /etc/letsencrypt/live/{subdomain}.tnnl.to/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{subdomain}.tnnl.to/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    root /var/www/html;
{auth_config}
    # Serve HTML for browser requests (no Upgrade header)
    location = / {{
        if ($http_upgrade = '') {{
            rewrite ^ /{subdomain}.html last;
        }}
        # WebSocket upgrade requests go to proxy
        proxy_pass http://127.0.0.1:{port};
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
    }}
}}
"#,
            subdomain = subdomain,
            port = port,
            auth_config = auth_config
        );

        // Ensure nginx directory exists
        tokio::fs::create_dir_all(NGINX_CONF_DIR).await.ok();

        // First, create HTTP-only config for certificate provisioning
        let http_only_config = format!(
            r#"server {{
    listen 80;
    listen [::]:80;
    server_name {subdomain}.tnnl.to;

    # ACME challenge location for certbot
    location /.well-known/acme-challenge/ {{
        root /var/www/certbot;
    }}

    # Temporary: serve content over HTTP
    root /var/www/html;
    location / {{
        return 200 'Certificate provisioning in progress...';
        add_header Content-Type text/plain;
    }}
}}
"#,
            subdomain = subdomain
        );

        // Write HTTP-only config
        let config_path = format!("/etc/nginx/sites-available/{}.tnnl.to", subdomain);
        let mut child = Command::new("sudo")
            .args(&["tee", &config_path])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(http_only_config.as_bytes())?;
        }
        child.wait()?;

        // Enable site by creating symlink in sites-enabled using sudo
        let enabled_path = format!("/etc/nginx/sites-enabled/{}.tnnl.to", subdomain);
        Command::new("sudo")
            .args(&["ln", "-sf", &config_path, &enabled_path])
            .output()?;

        // Reload Nginx with HTTP-only config
        self.reload_nginx().await?;

        // Request SSL certificate for this subdomain
        self.request_ssl_certificate(subdomain).await?;

        // Now write the full config with HTTPS
        let mut child = Command::new("sudo")
            .args(&["tee", &config_path])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(config.as_bytes())?;
        }
        child.wait()?;

        // Create client HTML file with pre-configured WebSocket URL
        self.create_client_html(subdomain).await?;

        // Create htpasswd file if password is set
        if let Some(password) = &tunnel.password {
            self.create_htpasswd(subdomain, password).await?;
        }

        // Reload Nginx with full HTTPS config
        self.reload_nginx().await?;

        println!("[Nginx] Configuration created for {}.tnnl.to", subdomain);
        Ok(())
    }

    /// Request SSL certificate for a subdomain using certbot
    async fn request_ssl_certificate(&self, subdomain: &str) -> anyhow::Result<()> {
        let domain = format!("{}.tnnl.to", subdomain);

        println!("[Nginx] Requesting SSL certificate for {}...", domain);

        // Check if certificate already exists
        let cert_path = format!("/etc/letsencrypt/live/{}/fullchain.pem", domain);
        if Path::new(&cert_path).exists() {
            println!("[Nginx] SSL certificate already exists for {}", domain);
            return Ok(());
        }

        // Ensure certbot webroot directory exists
        tokio::fs::create_dir_all("/var/www/certbot").await.ok();

        // Request certificate using certbot with webroot plugin
        let output = Command::new("sudo")
            .args(&[
                "certbot",
                "certonly",
                "--webroot",
                "--webroot-path", "/var/www/certbot",
                "-d", &domain,
                "--non-interactive",
                "--agree-tos",
                "--email", "admin@tnnl.to",
                "--keep-until-expiring"
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to obtain SSL certificate for {}: {}",
                domain,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        println!("[Nginx] SSL certificate obtained for {}", domain);
        Ok(())
    }

    /// Create client HTML file with pre-configured WebSocket URL
    async fn create_client_html(&self, subdomain: &str) -> anyhow::Result<()> {
        // Read template client.html
        let template_path = "/opt/tnnl/client.html";
        if !Path::new(template_path).exists() {
            println!("[Nginx] Warning: client.html template not found at {}", template_path);
            return Ok(()); // Don't fail if template missing
        }

        let template = tokio::fs::read_to_string(template_path).await?;

        // Replace placeholder WebSocket URL with this tunnel's HTTPS URL
        let customized = template
            .replace("ws://192.168.1.100:9001", &format!("wss://{}.tnnl.to", subdomain))
            .replace("placeholder=\"ws://192.168.1.100:9001\"",
                    &format!("placeholder=\"wss://{}.tnnl.to\"", subdomain));

        // Write to /var/www/html
        let html_path = format!("/var/www/html/{}.html", subdomain);
        tokio::fs::write(&html_path, customized).await?;

        println!("[Nginx] Created client HTML at {}", html_path);
        Ok(())
    }

    /// Remove tunnel configuration
    pub async fn remove_tunnel_config(&self, subdomain: &str) -> anyhow::Result<()> {
        println!("[Nginx] Removing configuration for tunnel: {}", subdomain);

        // Remove symlink from sites-enabled
        let enabled_path = format!("/etc/nginx/sites-enabled/{}.tnnl.to", subdomain);
        if Path::new(&enabled_path).exists() {
            tokio::fs::remove_file(&enabled_path).await?;
        }

        // Remove config file from sites-available
        let config_path = format!("/etc/nginx/sites-available/{}.tnnl.to", subdomain);
        if Path::new(&config_path).exists() {
            tokio::fs::remove_file(&config_path).await?;
        }

        // Remove client HTML
        let html_path = format!("/var/www/html/{}.html", subdomain);
        if Path::new(&html_path).exists() {
            tokio::fs::remove_file(&html_path).await?;
        }

        // Remove htpasswd file
        let passwd_path = format!("{}/{}.htpasswd", NGINX_PASSWD_DIR, subdomain);
        if Path::new(&passwd_path).exists() {
            tokio::fs::remove_file(&passwd_path).await?;
        }

        // Delete SSL certificate
        self.delete_ssl_certificate(subdomain).await.ok();

        // Reload Nginx
        self.reload_nginx().await?;

        println!("[Nginx] Configuration removed for {}.tnnl.to", subdomain);
        Ok(())
    }

    /// Delete SSL certificate for a subdomain
    async fn delete_ssl_certificate(&self, subdomain: &str) -> anyhow::Result<()> {
        let domain = format!("{}.tnnl.to", subdomain);

        println!("[Nginx] Deleting SSL certificate for {}...", domain);

        // Use certbot to delete the certificate
        let output = Command::new("sudo")
            .args(&[
                "certbot",
                "delete",
                "--cert-name", &domain,
                "--non-interactive"
            ])
            .output()?;

        if !output.status.success() {
            eprintln!("[Nginx] Warning: Failed to delete certificate for {}: {}",
                domain, String::from_utf8_lossy(&output.stderr));
        } else {
            println!("[Nginx] SSL certificate deleted for {}", domain);
        }

        Ok(())
    }

    /// Create htpasswd file for HTTP Basic Auth
    /// Always uses "tnnl" as the username for simplicity
    async fn create_htpasswd(&self, subdomain: &str, password: &str) -> anyhow::Result<()> {
        let passwd_path = format!("{}/{}.htpasswd", NGINX_PASSWD_DIR, subdomain);

        // Use htpasswd command to create the file
        // htpasswd -bc /path/to/file username password
        let output = Command::new("sudo")
            .args(&["htpasswd", "-bc", &passwd_path, "tnnl", password])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to create htpasswd file: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Reload Nginx configuration
    async fn reload_nginx(&self) -> anyhow::Result<()> {
        // First validate the configuration
        let test_output = Command::new("sudo")
            .args(&["nginx", "-t"])
            .output()?;

        if !test_output.status.success() {
            return Err(anyhow::anyhow!(
                "Nginx configuration test failed: {}",
                String::from_utf8_lossy(&test_output.stderr)
            ));
        }

        // Then reload using systemctl
        let output = Command::new("sudo")
            .args(&["systemctl", "reload", "nginx"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to reload Nginx: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }
}
