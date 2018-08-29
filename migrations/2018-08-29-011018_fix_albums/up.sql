CREATE SEQUENCE photo_albums_id_seq;
ALTER TABLE photo_albums ALTER COLUMN id SET DEFAULT nextval('photo_albums_id_seq');
