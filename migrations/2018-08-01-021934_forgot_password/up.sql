CREATE TABLE password_resets (
  uuid VARCHAR(32) PRIMARY KEY
  , user_id INTEGER NOT NULL REFERENCES users
  , created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
