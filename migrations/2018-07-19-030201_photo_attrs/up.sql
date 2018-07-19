-- KV triples table for attaching extra info (filename, tag, etc) to photos
CREATE TABLE photo_attrs (
  photo_id INTEGER NOT NULL REFERENCES photos
  , key VARCHAR(30) NOT NULL
  , value VARCHAR(100) NOT NULL
  , updated_at TIMESTAMP NOT NULL DEFAULT NOW()
  , PRIMARY KEY (photo_id, key)
);

-- forgot this when creating the photos table...
SELECT diesel_manage_updated_at('photos');
SELECT diesel_manage_updated_at('photo_attrs');
