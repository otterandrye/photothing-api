use chrono::NaiveDateTime;
use chrono::naive::serde::ts_seconds;
use diesel::Connection;
use std::collections::HashMap;
use std::iter::Iterator;

use auth::User;
use db::DbConn;
use db::photo::{NewPhotoAttr, NewPhoto, Photo as DbPhoto, PhotoAttr, AttributeKeyValue};
use errors::ApiError;
use s3::{sign_upload, S3Access, UploadRequest, UploadResponse};

// User-facing photo structure
#[derive(Serialize, Debug, PartialEq)]
pub struct Photo {
    uuid: String,
    present: bool,
    #[serde(with = "ts_seconds")]
    created_at: NaiveDateTime,
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
    let txn = db.transaction::<_, _, _>(|| {
        let photo = NewPhoto::new(user);
        let photo = photo.insert(db)?;
        let filename_attr = NewPhotoAttr::new(&photo, filename);
        let filename_attr = filename_attr.insert(db)?;

        let photo = Photo::new(photo, vec![filename_attr]);
        let upload = sign_upload(s3, &user.uuid, upload);

        Ok(PendingUpload { photo, upload })
    });
    ApiError::server_error(txn)
}

pub fn user_photos(user: &User, db: &DbConn) -> Result<Vec<Photo>, ApiError> {
    let photos = ApiError::server_error(DbPhoto::by_user(db, user))?;
    Ok(photos.into_iter()
        .map(|(p, a)| Photo::new(p, a))
        .collect())
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

        let user_photos = user_photos(&user, &db)?;
        assert_eq!(user_photos, vec![pending_upload.photo]);

        Ok(())
    }
}
