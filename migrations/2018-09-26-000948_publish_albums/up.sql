CREATE TABLE published_albums (
  id SERIAL PRIMARY KEY
  , album_id INTEGER NOT NULL REFERENCES photo_albums
  , user_id INTEGER NOT NULL REFERENCES users
  , active BOOL NOT NULL DEFAULT TRUE -- to allow easy un-publishing while keeping url
  , created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
