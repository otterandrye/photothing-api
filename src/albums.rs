use chrono::prelude::*;
use db::{DbConn, Pagination, Page};
use db::user::User;
use db::album::Album as DbAlbum;
use db::album::AlbumMembership;
use db::photo::Photo as DbPhoto;
use errors::ApiError;

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct AlbumEntry {
    photo: i32, // TODO: figure out how this should work to get to a user photo
    ordering: Option<i16>,
    caption: Option<String>,
    updated_at: DateTime<Utc>
}

impl AlbumEntry {
    fn new(photo: DbPhoto, album: AlbumMembership) -> Self {
        AlbumEntry {
            photo: photo.id,
            ordering: album.ordering,
            caption: album.caption,
            updated_at: album.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Album {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub photos: Page<AlbumEntry>
}

impl Album {
    fn new(album: DbAlbum, photos: Page<AlbumEntry>) -> Self {
        Album {
           id: album.id, created_at: album.created_at, name: album.name, photos
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
    Ok(Album::new(album, Page::empty()))
}

pub fn fetch_album(db: &DbConn, user: &User, id: i32, page: Pagination) -> AlbumResult {
    let album = fetch_db_album(&db, &user, id)?;
    load_photos_page(db, album, page)
}

pub fn add_photos_to_album(
    db: &DbConn, user: &User, id: i32, photo_ids: Vec<i32>
) -> AlbumResult {
    let album = fetch_db_album(&db, &user, id)?;
    ApiError::server_error(album.add_photos(&db, &photo_ids))?;
    load_photos(&db, album)
}

pub fn remove_photos_from_album(
    db: &DbConn, user: &User, id: i32, photo_ids: Vec<i32>
) -> AlbumResult {
    let album = fetch_db_album(&db, &user, id)?;
    ApiError::server_error(album.remove_photos(&db, &photo_ids))?;
    load_photos(&db, album)
}

pub fn user_albums(db: &DbConn, user: &User, page: Pagination) -> Result<Page<Album>, ApiError> {
    let db_albums = ApiError::server_error(DbAlbum::for_user(&db, &user, page))?;
    let albums = db_albums.map(|a| Album::new(a, Page::empty()));
    Ok(albums)
}

fn fetch_db_album(db: &DbConn, user: &User, id: i32) -> Result<DbAlbum, ApiError> {
    let album = ApiError::server_error(DbAlbum::by_id(&db, &user, id))?;
    ApiError::not_found(album, format!("could not find album with id={}", id))
}

fn load_photos(db: &DbConn, album: DbAlbum) -> AlbumResult {
    load_photos_page(db, album, Pagination::first())
}

fn load_photos_page(db: &DbConn, album: DbAlbum, page: Pagination) -> AlbumResult {
    let photos = ApiError::server_error(album.get_photos(db, page))?;
    let photos = photos.map(|(a, p)| AlbumEntry::new(p, a));
    Ok(Album::new(album, photos))
}

#[cfg(test)]
mod functest {
    use db::test_db;
    use db::user::NewUser;
    use db::photo::NewPhoto;
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

        let by_user = user_albums(&db, &user, Pagination::first())
            .expect("couldn't find user's albums");
        assert_eq!(by_user.items.len(), 1);
        assert_eq!(&album, by_user.items.get(0).unwrap());

        let by_id = fetch_album(&db, &user, album.id, Pagination::first())
            .expect("couldn't fetch album by id");
        assert_eq!(&album, &by_id);

        fetch_album(&db, &user, 392390, Pagination::first())
            .expect_err("got album back for nonsense id");

        add_photos_to_album(&db, &user, album.id, vec![32018])
            .expect_err("can't add nonsense photo id to album");

        let photo = NewPhoto::new(&user);
        let photo = photo.insert(&db).expect("failed to insert photo");
        let album = add_photos_to_album(&db, &user, album.id, vec![photo.id])
            .expect("couldn't add photo to album");
        assert_eq!(album.photos.items.len(), 1, "didn't add photo");

        let album = add_photos_to_album(&db, &user, album.id, vec![photo.id])
            .expect("adding same photo again doesn't error out");
        assert_eq!(album.photos.items.len(), 1);

        let noop_remove = remove_photos_from_album(&db, &user, album.id, vec![3002101])
            .expect("got err when removing non-existent photos from album");
        assert_eq!(&album, &noop_remove);

        let album = remove_photos_from_album(&db, &user, album.id, vec![photo.id])
            .expect("couldn't remove photo from album");
        assert_eq!(album.photos.items.len(), 0, "didn't remove photo");
    }
}
