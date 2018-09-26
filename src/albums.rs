use chrono::prelude::*;
use db::{DbConn, Pagination, Page};
use db::user::User;
use db::album::Album as DbAlbum;
use db::album::AlbumMembership;
use db::photo::Photo as DbPhoto;
use db::photo::PhotoAttr;
use errors::ApiError;
use photos::Photo;
use s3::S3Access;

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct AlbumEntry {
    pub photo: Photo,
    ordering: Option<i16>,
    pub caption: Option<String>,
    updated_at: DateTime<Utc>
}

impl AlbumEntry {
    fn new(user: &User, s3: &S3Access, photo: DbPhoto, album: AlbumMembership, attrs: Vec<PhotoAttr>) -> Self {
        AlbumEntry {
            photo: Photo::new(user, s3, photo, attrs),
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
    let album = DbAlbum::create(&db, user, details.name())?;
    Ok(Album::new(album, Page::empty()))
}

pub fn fetch_album(db: &DbConn, user: &User, s3: &S3Access, id: i32, page: Pagination) -> AlbumResult {
    let album = fetch_db_album(&db, &user, id)?;
    load_photos_page(user, s3, db, album, page)
}

pub fn add_photos_to_album(
    db: &DbConn, user: &User, s3: &S3Access, id: i32, photo_ids: Vec<i32>
) -> AlbumResult {
    let album = fetch_db_album(&db, &user, id)?;
    album.add_photos(&db, &photo_ids)?;
    load_photos(user, s3, &db, album)
}

pub fn remove_photos_from_album(
    db: &DbConn, user: &User, s3: &S3Access, id: i32, photo_ids: Vec<i32>
) -> AlbumResult {
    let album = fetch_db_album(&db, &user, id)?;
    album.remove_photos(&db, &photo_ids)?;
    load_photos(user, s3, &db, album)
}

pub fn user_albums(db: &DbConn, user: &User, page: Pagination) -> Result<Page<Album>, ApiError> {
    let db_albums = DbAlbum::for_user(&db, &user, page)?;
    let albums = db_albums.map(|a| Album::new(a, Page::empty()));
    Ok(albums)
}

fn fetch_db_album(db: &DbConn, user: &User, id: i32) -> Result<DbAlbum, ApiError> {
    let album = ApiError::server_error(DbAlbum::by_id(&db, &user, id))?;
    ApiError::not_found(album, format!("could not find album with id={}", id))
}

fn load_photos(user: &User, s3: &S3Access, db: &DbConn, album: DbAlbum) -> AlbumResult {
    load_photos_page(user, s3, db, album, Pagination::first())
}

fn load_photos_page(user: &User, s3: &S3Access, db: &DbConn, album: DbAlbum, page: Pagination) -> AlbumResult {
    let photos = album.get_photos(db, page)?;
    // use a closure to destructure the :( return type we get from the db code and curry the
    // s3 + user params
    let decorated_photo = |(p, m, a)| AlbumEntry::new(user, s3, p, m, a);
    let photos = photos.map(decorated_photo);
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
        let s3 = S3Access::new("fake_bucket".into(), "foo.com".into(), None);

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

        let by_id = fetch_album(&db, &user, &s3, album.id, Pagination::first())
            .expect("couldn't fetch album by id");
        assert_eq!(&album, &by_id);

        fetch_album(&db, &user, &s3, 392390, Pagination::first())
            .expect_err("got album back for nonsense id");

        add_photos_to_album(&db, &user, &s3, album.id, vec![32018])
            .expect_err("can't add nonsense photo id to album");

        let photo = NewPhoto::new(&user);
        let photo = photo.insert(&db).expect("failed to insert photo");
        let album = add_photos_to_album(&db, &user, &s3, album.id, vec![photo.id])
            .expect("couldn't add photo to album");
        assert_eq!(album.photos.items.len(), 1, "didn't add photo");

        let album = add_photos_to_album(&db, &user, &s3, album.id, vec![photo.id])
            .expect("adding same photo again doesn't error out");
        assert_eq!(album.photos.items.len(), 1);

        let noop_remove = remove_photos_from_album(&db, &user, &s3, album.id, vec![3002101])
            .expect("got err when removing non-existent photos from album");
        assert_eq!(&album, &noop_remove);

        let album = remove_photos_from_album(&db, &user, &s3, album.id, vec![photo.id])
            .expect("couldn't remove photo from album");
        assert_eq!(album.photos.items.len(), 0, "didn't remove photo");
    }
}
