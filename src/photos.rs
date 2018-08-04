use chrono::prelude::*;
use chrono::serde::ts_seconds;
use diesel::Connection;
use std::collections::HashMap;

use auth::User;
use db::{DbConn, Pagination, Page};
use db::photo::{NewPhotoAttr, NewPhoto, Photo as DbPhoto, PhotoAttr, AttributeKeyValue};
use errors::ApiError;
use s3::{sign_upload, S3Access, UploadRequest, UploadResponse};

// User-facing photo structure
#[derive(Serialize, Debug, PartialEq)]
pub struct Photo {
    uuid: String,
    present: bool,
    #[serde(with = "ts_seconds")]
    created_at: DateTime<Utc>,
    attributes: HashMap<String, String>,
}

impl Photo {
    fn new(photo: DbPhoto, attributes: Vec<PhotoAttr>) -> Photo {
        let mut attr_map = HashMap::new();
        for attr in attributes.into_iter() {
            attr_map.insert(attr.key, attr.value);
        }
        Photo {
            uuid: photo.uuid,
            present: photo.present.unwrap_or(false),
            created_at: photo.created_at,
            attributes: attr_map,
        }
    }
}

#[derive(Serialize)]
pub struct PendingUpload {
    photo: Photo,
    upload: UploadResponse,
}

pub fn create_photo(user: &User, db: &DbConn, s3: &S3Access, upload: UploadRequest)
    -> Result<PendingUpload, ApiError>
{
    let filename = ApiError::bad_request(AttributeKeyValue::new("filename", &upload.filename))?;

    use diesel::result::Error;

    let txn = db.transaction::<_, Error, _>(|| {
        let photo = NewPhoto::new(user);
        let photo = photo.insert(db)?;
        let filename_attr = NewPhotoAttr::new(&photo, filename);
        let filename_attr = filename_attr.insert(db)?;

        let photo = Photo::new(photo, vec![filename_attr]);
        let upload = sign_upload(s3, &user.uuid, upload, &photo.uuid);

        Ok(PendingUpload { photo, upload })
    });
    ApiError::server_error(txn)
}

pub fn user_photos(user: &User, db: &DbConn, pagination: Pagination) -> Result<Page<Photo>, ApiError> {
    let page = ApiError::server_error(DbPhoto::by_user(db, user, pagination))?;
    Ok(page.map(|(p, a)| Photo::new(p, a)))
}

#[cfg(test)]
mod test {
    use dotenv;

    use db::{DbConn, init_db_pool};
    use db::user::NewUser;
    use s3::UploadRequest;
    use super::*;

    fn setup() -> (User, S3Access, DbConn) {
        dotenv::dotenv().ok();
        let s3 = S3Access::new("fake_bucket".into(), "foo.com".into(), None);
        let pool = init_db_pool();
        let db = DbConn(pool.get().expect("couldn't connect to db"));
        let user = NewUser::fake("e");
        let user = user.insert(&db).expect("couldn't make user");
        (user, s3, db)
    }

    #[test]
    fn photo_create_get() -> Result<(), ApiError> {
        let (user, s3, db) = setup();
        let upload = UploadRequest::fake();

        let pending_upload = create_photo(&user, &db, &s3, upload)?;
        assert!(!pending_upload.photo.present);

        let photos = user_photos(&user, &db, Pagination::first())?;
        assert_eq!(photos.items, vec![pending_upload.photo]);
        assert_eq!(photos.remaining, 0);
        assert_eq!(photos.key, None);
        match photos.next_key {
            Some(id) if id > 0 => {},
            _ => assert!(false, "Didn't get next key: {:?}", photos.next_key),
        }

        let key = photos.next_key.unwrap();
        let second_page = user_photos(&user, &db, Pagination::page(key))?;
        assert!(second_page.items.is_empty());
        assert_eq!(second_page.remaining, 0);
        assert_eq!(second_page.key, Some(key), "user-supplied key");
        assert_eq!(second_page.next_key, None, "next key");

        for _ in 0..50 {
            let upload = UploadRequest::fake();
            create_photo(&user, &db, &s3, upload)?;
        }
        let photos = user_photos(&user, &db, Pagination::first())?;
        assert_eq!(photos.items.len() as i64, Pagination::first().per_page);
        assert_eq!(photos.remaining, 21); // 51 uploaded, got first 30

        let mut small_pg = Pagination::first();
        small_pg.per_page = 5;
        let fewer = user_photos(&user, &db, small_pg)?;
        assert_eq!(fewer.items.len() as i64, small_pg.per_page);
        assert_eq!(fewer.remaining, 46); // 51 uploaded, asked for 5

        Ok(())
    }
}
