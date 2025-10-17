use anyhow::Result;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions, Row};
use uuid::Uuid;
use crate::tunnel::Tunnel;

pub type DbPool = Pool<Postgres>;

/// Initialize Postgres connection pool for Supabase
pub async fn init_pool(database_url: &str) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // Note: Schema is managed in Supabase SQL editor (see supabase-schema.sql)
    // No automatic migrations - run the SQL file in Supabase dashboard

    Ok(pool)
}

/// Get or create user record
/// For Supabase, users are managed in auth.users, so we don't need to create them here
/// We just validate the user_id exists and return it
/// If using tunnels.user_id FK to auth.users, this is just a passthrough
pub async fn get_or_create_user(pool: &DbPool, user_id: Uuid, _email: &str) -> Result<Uuid> {
    // With Supabase, users are created by Supabase Auth
    // We just validate the user exists in auth.users
    // The tunnels.user_id FK will enforce this

    // For now, just return the user_id as-is
    // The FK constraint will fail if the user doesn't exist in auth.users
    Ok(user_id)
}

/// Create tunnel record in Supabase
pub async fn create_tunnel_record(pool: &DbPool, tunnel: &Tunnel) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO tunnels (id, subdomain, user_id, is_custom, port, password, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    )
    .bind(tunnel.id)
    .bind(&tunnel.subdomain)
    .bind(tunnel.user_id)
    .bind(tunnel.is_custom)
    .bind(tunnel.port as i32)
    .bind(&tunnel.password)
    .bind(tunnel.created_at)
    .bind(tunnel.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(dead_code)]
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
            Ok(Some(Tunnel {
                id: r.try_get("id")?,
                subdomain: r.try_get("subdomain")?,
                user_id: r.try_get("user_id")?,
                is_custom: r.try_get("is_custom")?,
                port: r.try_get::<i32, _>("port")? as u16,
                password: r.try_get("password")?,
                created_at: r.try_get("created_at")?,
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

#[allow(dead_code)]
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
#[allow(dead_code)]
pub async fn get_user_tunnels(pool: &DbPool, user_id: Uuid) -> Result<Vec<Tunnel>> {
    let rows = sqlx::query(
        r#"
        SELECT id, subdomain, user_id, is_custom, port, password, created_at
        FROM tunnels
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut tunnels = Vec::new();
    for r in rows {
        tunnels.push(Tunnel {
            id: r.try_get("id")?,
            subdomain: r.try_get("subdomain")?,
            user_id: r.try_get("user_id")?,
            is_custom: r.try_get("is_custom")?,
            port: r.try_get::<i32, _>("port")? as u16,
            password: r.try_get("password")?,
            created_at: r.try_get("created_at")?,
        });
    }

    Ok(tunnels)
}

/// Store SSH public key for a user
/// Creates or updates user_profile with SSH key
pub async fn store_ssh_public_key(pool: &DbPool, user_id: Uuid, ssh_public_key: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_profiles (id, ssh_public_key)
        VALUES ($1, $2)
        ON CONFLICT (id) DO UPDATE SET
            ssh_public_key = $2,
            updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(user_id)
    .bind(ssh_public_key)
    .execute(pool)
    .await?;

    Ok(())
}
