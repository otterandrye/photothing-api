use chrono::prelude::*;
use harsh::Harsh;

use db::{DbConn, Pagination, Page};
use db::user::User;
use db::album::Album as DbAlbum;
use db::album::AlbumMembership;
use db::photo::Photo as DbPhoto;
use db::photo::PhotoAttr;
use db::publishing::PublishedAlbum;
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

#[derive(FromForm)]
pub struct ToggleActive {
    active: bool
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct UrlFriendlyAlbum {
    album_id: i32,
    created_at: DateTime<Utc>,
    hash: String,
    active: bool,
}

impl UrlFriendlyAlbum {
    fn new(harsh: &Harsh, published: PublishedAlbum) -> Self {
        let hash = harsh.encode_hex(&published.id.to_string()[..])
            .expect("hashing album publish id failed");
        UrlFriendlyAlbum {
            hash,
            album_id: published.album_id,
            active: published.active,
            created_at: published.created_at,
        }
    }

    /// return a published album id given a user-supplied string which may be decode-able
    fn decode_id(harsh: &Harsh, given_id: &str) -> Result<i32, ApiError> {
        // note: never tell the user that decoding their input failed, just 404
        let as_str = ApiError::not_found(harsh.decode_hex(given_id), format!("{}", given_id))?;
        match as_str.parse::<i32>() {
            Ok(id) => Ok(id),
            Err(_) => ApiError::not_found::<i32>(None, format!("{}", given_id)),
        }
    }
}

/// Helper to make the 404s in these code paths consistent no matter what doesn't work
fn not_found<T>(album: Option<T>, hash_id: &str) -> Result<T, ApiError> {
    ApiError::not_found(album, format!("no published album with id={}", hash_id))
}

pub fn publish_album(db: &DbConn, user: &User, harsh: &Harsh, id: i32) -> Result<UrlFriendlyAlbum, ApiError> {
    let album = fetch_db_album(&db, &user, id)?;
    let published = PublishedAlbum::publish(&db, album)?;
    Ok(UrlFriendlyAlbum::new(harsh, published))
}

fn get_published_album(db: &DbConn, harsh: &Harsh, hash_id: &String) -> Result<PublishedAlbum, ApiError> {
    let published_album_id = UrlFriendlyAlbum::decode_id(&harsh, hash_id)?;
    let album = PublishedAlbum::by_id(db, published_album_id)?;
    not_found(album, hash_id)
}

fn get_users_published_album(db: &DbConn, user: &User, harsh: &Harsh, hash_id: &String) -> Result<PublishedAlbum, ApiError> {
    let album = get_published_album(db, harsh, hash_id)?;
    if album.user_id != user.id {
        not_found(None, hash_id)
    } else {
        Ok(album)
    }
}

pub fn delete_published_album(db: &DbConn, user: &User, harsh: &Harsh, hash_id: String) -> Result<(), ApiError> {
    let album = get_users_published_album(db, user, harsh, &hash_id)?;
    album.delete(db)?;
    Ok(())
}

pub fn toggle_published_album(db: &DbConn, user: &User, harsh: &Harsh, hash_id: String, active: ToggleActive) -> Result<(), ApiError> {
    let album = get_users_published_album(db, user, harsh, &hash_id)?;
    album.set_active(db, active.active)?;
    Ok(())
}

pub fn user_published_albums(db: &DbConn, user: &User, harsh: &Harsh) -> Result<Vec<UrlFriendlyAlbum>, ApiError> {
    let albums = PublishedAlbum::for_user(db, user)?;
    Ok(albums.into_iter().map(|a| UrlFriendlyAlbum::new(&harsh, a)).collect())
}

pub fn get_published_photos(db: &DbConn, s3: &S3Access, harsh: &Harsh, hash_id: String, page: Pagination) -> AlbumResult {
    let published = get_published_album(db, harsh, &hash_id)?;
    if !published.active {
        return not_found(None, &hash_id);
    }
    let user = not_found(User::by_id(db, published.user_id)?, &hash_id)?;
    let album = not_found(DbAlbum::by_id(db, &user, published.album_id)?, &hash_id)?;
    load_photos_page(&user, s3, db, album, page)
}

#[cfg(test)]
mod functest {
    use db::test_db;
    use db::user::NewUser;
    use db::photo::NewPhoto;
    use harsh::HarshBuilder;
    use super::*;

    #[test]
    fn url_friendliness() {
        let harsh = HarshBuilder::new().salt("foo").init().expect("harsh init failed (harsh)");

        let friendly = UrlFriendlyAlbum::new(&harsh, PublishedAlbum::fake());
        assert_eq!("n82", friendly.hash);

        let decoded = UrlFriendlyAlbum::decode_id(&harsh, &friendly.hash).expect("decoded ok");
        assert_eq!(PublishedAlbum::fake().id, decoded);

        match UrlFriendlyAlbum::decode_id(&harsh, "nonsense") {
            Ok(_) => assert!(false, "got something back decoding nonsense"),
            Err(e) => assert!(e.is_user_error(), format!("got non user-error back: {}", e.status)),
        }
    }

    #[test]
    #[should_panic(expected = "attempt to multiply with overflow")]
    fn document_harsher_bug() {
        // kind of scary that Harsh panics here, but I don't think a malicious user could get up
        // to any real trouble other than making a lot of noise by attacking this...
        let harsh = HarshBuilder::new().salt("foo").init().expect("harsh init failed (harsh)");
        assert!(UrlFriendlyAlbum::decode_id(&harsh, "nonsense!!#$!)(&#(*)").is_err());
    }

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

    #[ignore]
    #[test]
    fn album_publish_workflow() {
        let db = test_db();
        let user = NewUser::fake("album_publish@gmail.com").insert(&db)
            .expect("couldn't make user");
        let mut u2 = user.clone();
        u2.id = 999;
        let s3 = S3Access::new("fake_bucket".into(), "foo.com".into(), None);
        let harsh = HarshBuilder::new().salt("foo").init().expect("harsh init failed (harsh)");

        let name = Some("published album".to_owned());
        let album = create_album(&db, &user, NewAlbum { name: name.clone() })
            .expect("couldn't create album");
        let photo = NewPhoto::new(&user);
        let photo = photo.insert(&db).expect("failed to insert photo");
        add_photos_to_album(&db, &user, &s3, album.id, vec![photo.id])
            .expect("couldn't add photo to album");

        publish_album(&db, &user, &harsh, 29098).expect_err("can't publish rando id");
        publish_album(&db, &u2, &harsh, album.id).expect_err("can't publish somebody else's album");

        let published = publish_album(&db, &user, &harsh, album.id)
            .expect("publish worked");
        assert!(published.active);

        let user_published = user_published_albums(&db, &user, &harsh).expect("album fetch ok");
        assert_eq!(user_published.get(0).unwrap(), &published);

        let album = get_published_photos(&db, &s3, &harsh, published.hash.clone(), Pagination::first())
            .expect("get published");
        let published_photo = &album.photos.items.get(0).unwrap().photo;
        assert_eq!(published_photo.id, photo.id, "right photo in published album");

        let other_user_pub = user_published_albums(&db, &u2, &harsh).expect("album fetch ok");
        assert!(other_user_pub.is_empty());

        let disabled = ToggleActive { active: false };
        toggle_published_album(&db, &user, &harsh, published.hash.clone(), disabled).expect("toggled off");
        get_published_photos(&db, &s3, &harsh, published.hash.clone(), Pagination::first())
            .expect_err("album not published anymore");

        let enabled = ToggleActive { active: true };
        toggle_published_album(&db, &user, &harsh, published.hash.clone(), enabled).expect("toggled back on");
        let album = get_published_photos(&db, &s3, &harsh, published.hash.clone(), Pagination::first())
            .expect("album re-published");
        assert_eq!(album.photos.items.len(), 1);

        delete_published_album(&db, &u2, &harsh, published.hash.clone()).expect_err("can't delete other users albums");
        delete_published_album(&db, &user, &harsh, published.hash.clone()).expect("delete ok");
        get_published_photos(&db, &s3, &harsh, published.hash.clone(), Pagination::first())
            .expect_err("album not published anymore");
    }
}
