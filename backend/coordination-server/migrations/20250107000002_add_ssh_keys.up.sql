-- Add SSH public key column to users table
ALTER TABLE users ADD COLUMN ssh_public_key TEXT;

-- Create index for SSH key lookups
CREATE INDEX IF NOT EXISTS idx_users_ssh_key ON users(ssh_public_key);
