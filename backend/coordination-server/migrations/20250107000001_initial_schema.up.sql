-- Create users table
-- Note: This tracks users from Supabase auth
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create index on email for faster lookups
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Create tunnels table
CREATE TABLE IF NOT EXISTS tunnels (
    id TEXT PRIMARY KEY,
    subdomain TEXT NOT NULL UNIQUE,
    user_id TEXT NOT NULL,
    is_custom INTEGER NOT NULL DEFAULT 0,
    port INTEGER NOT NULL,
    password TEXT, -- Optional HTTP Basic Auth password
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_connected_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_tunnels_subdomain ON tunnels(subdomain);
CREATE INDEX IF NOT EXISTS idx_tunnels_user_id ON tunnels(user_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_port ON tunnels(port);
