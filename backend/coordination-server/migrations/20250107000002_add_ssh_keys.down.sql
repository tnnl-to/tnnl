-- Remove SSH key index
DROP INDEX IF EXISTS idx_users_ssh_key;

-- Remove SSH public key column from users table
ALTER TABLE users DROP COLUMN ssh_public_key;
