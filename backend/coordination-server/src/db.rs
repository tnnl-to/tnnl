use anyhow::Result;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions, Row};
use uuid::Uuid;
use crate::tunnel::Tunnel;

pub type DbPool = Pool<Sqlite>;

/// Initialize database connection pool and run migrations
pub async fn init_pool(database_url: &str) -> Result<DbPool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}

/// Database operations for users
pub async fn get_or_create_user(pool: &DbPool, user_id: Uuid, email: &str) -> Result<()> {
    let user_id_str = user_id.to_string();

    // Use INSERT OR IGNORE to handle both new and existing users
    // If the email already exists, we just update the timestamp but keep the same ID
    // to avoid violating foreign key constraints with existing tunnels
    sqlx::query(
        "INSERT INTO users (id, email) VALUES ($1, $2)
         ON CONFLICT(email) DO UPDATE SET
         updated_at = CURRENT_TIMESTAMP"
    )
    .bind(&user_id_str)
    .bind(email)
    .execute(pool)
    .await?;

    Ok(())
}

/// Database operations for tunnels
pub async fn create_tunnel_record(pool: &DbPool, tunnel: &Tunnel) -> Result<()> {
    let id_str = tunnel.id.to_string();
    let user_id_str = tunnel.user_id.to_string();
    let created_at_str = tunnel.created_at.to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO tunnels (id, subdomain, user_id, is_custom, port, password, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    )
    .bind(&id_str)
    .bind(&tunnel.subdomain)
    .bind(&user_id_str)
    .bind(tunnel.is_custom)
    .bind(tunnel.port as i32)
    .bind(&tunnel.password)
    .bind(&created_at_str)
    .bind(&created_at_str)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_tunnel_by_subdomain(pool: &DbPool, subdomain: &str) -> Result<Option<Tunnel>> {
    let row = sqlx::query(
        r#"
        SELECT id, subdomain, user_id, is_custom, port, password, created_at
        FROM tunnels
        WHERE subdomain = $1
        "#
    )
    .bind(subdomain)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let id_str: String = r.try_get("id")?;
            let user_id_str: String = r.try_get("user_id")?;
            let created_at_str: String = r.try_get("created_at")?;

            Ok(Some(Tunnel {
                id: Uuid::parse_str(&id_str)?,
                subdomain: r.try_get("subdomain")?,
                user_id: Uuid::parse_str(&user_id_str)?,
                is_custom: r.try_get("is_custom")?,
                port: r.try_get::<i32, _>("port")? as u16,
                password: r.try_get("password")?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc),
            }))
        }
        None => Ok(None),
    }
}

pub async fn delete_tunnel_record(pool: &DbPool, subdomain: &str) -> Result<()> {
    sqlx::query(
        "DELETE FROM tunnels WHERE subdomain = $1"
    )
    .bind(subdomain)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_tunnel_last_connected(pool: &DbPool, subdomain: &str) -> Result<()> {
    sqlx::query(
        "UPDATE tunnels SET last_connected_at = CURRENT_TIMESTAMP WHERE subdomain = $1"
    )
    .bind(subdomain)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all tunnels for a user
pub async fn get_user_tunnels(pool: &DbPool, user_id: Uuid) -> Result<Vec<Tunnel>> {
    let user_id_str = user_id.to_string();

    let rows = sqlx::query(
        r#"
        SELECT id, subdomain, user_id, is_custom, port, password, created_at
        FROM tunnels
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(&user_id_str)
    .fetch_all(pool)
    .await?;

    let mut tunnels = Vec::new();
    for r in rows {
        let id_str: String = r.try_get("id")?;
        let user_id_str: String = r.try_get("user_id")?;
        let created_at_str: String = r.try_get("created_at")?;

        tunnels.push(Tunnel {
            id: Uuid::parse_str(&id_str)?,
            subdomain: r.try_get("subdomain")?,
            user_id: Uuid::parse_str(&user_id_str)?,
            is_custom: r.try_get("is_custom")?,
            port: r.try_get::<i32, _>("port")? as u16,
            password: r.try_get("password")?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc),
        });
    }

    Ok(tunnels)
}

/// Store SSH public key for a user
pub async fn store_ssh_public_key(pool: &DbPool, user_id: Uuid, ssh_public_key: &str) -> Result<()> {
    let user_id_str = user_id.to_string();

    sqlx::query(
        "UPDATE users SET ssh_public_key = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
    )
    .bind(ssh_public_key)
    .bind(&user_id_str)
    .execute(pool)
    .await?;

    Ok(())
}
