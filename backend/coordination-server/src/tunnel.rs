use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
#[allow(unused)]
pub struct Tunnel {
    pub id: Uuid,
    pub subdomain: String,
    pub user_id: Uuid,
    pub is_custom: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub port: u16, // Local port for forwarding
    pub password: Option<String>, // Optional HTTP Basic Auth password
}

pub struct TunnelManager {
    tunnels: Arc<RwLock<HashMap<String, Tunnel>>>, // subdomain -> tunnel
    ports: Arc<RwLock<HashMap<u16, Uuid>>>,         // port -> tunnel_id
    next_port: Arc<RwLock<u16>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            ports: Arc::new(RwLock::new(HashMap::new())),
            next_port: Arc::new(RwLock::new(10000)), // Start from port 10000
        }
    }

    /// Create a new tunnel with a random subdomain
    pub async fn create_random_tunnel(
        &self,
        user_id: Uuid,
        password: Option<String>,
    ) -> anyhow::Result<Tunnel> {
        // Generate random subdomain (adjective-noun-number pattern)
        let subdomain = generate_random_subdomain();
        self.create_tunnel(user_id, subdomain, false, password).await
    }

    /// Create a new tunnel with a custom subdomain
    #[allow(dead_code)]
    pub async fn create_custom_tunnel(
        &self,
        user_id: Uuid,
        subdomain: String,
        password: Option<String>,
    ) -> anyhow::Result<Tunnel> {
        // Validate subdomain
        if !is_valid_subdomain(&subdomain) {
            return Err(anyhow::anyhow!("Invalid subdomain format"));
        }

        // Check if already exists
        let tunnels = self.tunnels.read().await;
        if tunnels.contains_key(&subdomain) {
            return Err(anyhow::anyhow!("Subdomain already in use"));
        }
        drop(tunnels);

        self.create_tunnel(user_id, subdomain, true, password).await
    }

    async fn create_tunnel(
        &self,
        user_id: Uuid,
        subdomain: String,
        is_custom: bool,
        password: Option<String>,
    ) -> anyhow::Result<Tunnel> {
        // Allocate port
        let port = {
            let mut next_port = self.next_port.write().await;
            let port = *next_port;
            *next_port += 1;
            port
        };

        let tunnel = Tunnel {
            id: Uuid::new_v4(),
            subdomain: subdomain.clone(),
            user_id,
            is_custom,
            created_at: chrono::Utc::now(),
            port,
            password,
        };

        // Store tunnel
        {
            let mut tunnels = self.tunnels.write().await;
            tunnels.insert(subdomain.clone(), tunnel.clone());
        }

        {
            let mut ports = self.ports.write().await;
            ports.insert(port, tunnel.id);
        }

        Ok(tunnel)
    }

    /// Get tunnel by subdomain
    #[allow(dead_code)]
    pub async fn get_tunnel(&self, subdomain: &str) -> Option<Tunnel> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(subdomain).cloned()
    }

    /// Remove tunnel
    pub async fn remove_tunnel(&self, subdomain: &str) -> anyhow::Result<()> {
        let mut tunnels = self.tunnels.write().await;
        if let Some(tunnel) = tunnels.remove(subdomain) {
            let mut ports = self.ports.write().await;
            ports.remove(&tunnel.port);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Tunnel not found"))
        }
    }
}

fn generate_random_subdomain() -> String {
    use rand::seq::SliceRandom;
    use rand::Rng;

    let adjectives = vec![
        "happy", "fuzzy", "clever", "swift", "bright", "calm", "brave", "wild", "cool", "warm",
    ];
    let nouns = vec![
        "cat", "dog", "bird", "fish", "bear", "wolf", "fox", "deer", "owl", "lion",
    ];

    let mut rng = rand::thread_rng();
    let adj = adjectives.choose(&mut rng).unwrap();
    let noun = nouns.choose(&mut rng).unwrap();
    let num: u16 = rng.gen_range(1000..9999);

    format!("{}-{}-{}", adj, noun, num)
}

#[allow(dead_code)]
fn is_valid_subdomain(subdomain: &str) -> bool {
    // Subdomain must be 3-63 chars, lowercase alphanumeric and hyphens only
    if subdomain.len() < 3 || subdomain.len() > 63 {
        return false;
    }

    subdomain
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !subdomain.starts_with('-')
        && !subdomain.ends_with('-')
}
