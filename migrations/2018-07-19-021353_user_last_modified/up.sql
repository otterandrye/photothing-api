ALTER TABLE users ADD COLUMN updated_at TIMESTAMP NOT NULL DEFAULT NOW();
SELECT diesel_manage_updated_at('users');
