CREATE TABLE photo_albums (
  id INTEGER PRIMARY KEY
  , user_id INTEGER NOT NULL REFERENCES users
  , name TEXT
  , created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE album_membership (
  photo_id INTEGER NOT NULL REFERENCES photos
  , album_id INTEGER NOT NULL REFERENCES photo_albums
  , ordering SMALLINT
  , caption TEXT
  , updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  , PRIMARY KEY (photo_id, album_id)
);

SELECT diesel_manage_updated_at('album_membership');
