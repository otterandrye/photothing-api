use chrono::prelude::*;
use db::{DbConn, Pagination, Page};
use db::user::User;
use db::album::Album as DbAlbum;
use errors::ApiError;

#[derive(Serialize, Debug)]
pub struct Album {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub photos: Page<String> // TODO
}

impl Album {
    fn new(album: DbAlbum) -> Self {
        Album {
           id: album.id, created_at: album.created_at, name: album.name, photos: Page::empty()
        }
    }
}

type AlbumResult = Result<Album, ApiError>;

#[derive(FromForm)]
pub struct NewAlbum {
    name: Option<String>
}

impl NewAlbum {
    fn name(&self) -> Option<&str> {
        match self.name {
            Some(ref name) => Some(name),
            None => None,
        }
    }
}

pub fn create_album(db: &DbConn, user: &User, details: NewAlbum) -> AlbumResult {
    let album = ApiError::server_error(DbAlbum::create(&db, user, details.name()))?;
    Ok(Album::new(album))
}

pub fn fetch_album(db: &DbConn, user: &User, id: i32, page: Pagination) -> AlbumResult {
    unimplemented!()
}

pub fn add_photos_to_album(
    db: &DbConn, user: &User, id: i32, photo_ids: Vec<i32>
) -> AlbumResult {
    unimplemented!()
}

pub fn remove_photos_from_album(
    db: &DbConn, user: &User, id: i32, photo_ids: Vec<i32>
) -> AlbumResult {
    unimplemented!()
}

pub fn user_albums(db: &DbConn, user: &User) -> Result<Page<Album>, ApiError> {
    unimplemented!()
}

#[cfg(test)]
mod functest {
    use db::test_db;
    use db::user::NewUser;
    use super::*;

    #[test]
    fn album_crud() {
        let db = test_db();
        let user = NewUser::fake("album_crud@gmail.com").insert(&db)
            .expect("couldn't make user");

        let name = Some("baby's first album".to_owned());
        let album = create_album(&db, &user, NewAlbum { name: name.clone() })
            .expect("couldn't create album");
        assert_eq!(album.photos.remaining, 0);
        assert_eq!(album.photos.items.len(), 0);
        assert_eq!(&album.name, &name);
    }
}
