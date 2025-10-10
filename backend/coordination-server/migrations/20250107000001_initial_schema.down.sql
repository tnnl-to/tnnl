-- Drop triggers
DROP TRIGGER IF EXISTS update_tunnels_updated_at ON tunnels;
DROP TRIGGER IF EXISTS update_users_updated_at ON users;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop indexes
DROP INDEX IF EXISTS idx_tunnels_port;
DROP INDEX IF EXISTS idx_tunnels_user_id;
DROP INDEX IF EXISTS idx_tunnels_subdomain;
DROP INDEX IF EXISTS idx_users_email;

-- Drop tables (CASCADE will drop foreign key constraints)
DROP TABLE IF EXISTS tunnels CASCADE;
DROP TABLE IF EXISTS users CASCADE;
