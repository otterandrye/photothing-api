ALTER TABLE users DROP COLUMN updated_at;
DROP TRIGGER IF EXISTS set_updated_at ON users;
