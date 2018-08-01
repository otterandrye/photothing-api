use diesel::PgConnection;
use diesel::dsl::count;
use diesel::query_dsl::*;
use diesel::result::Error;

use s3::S3Access;
use errors::ApiError;

#[derive(Serialize)]
pub struct AdminContext {
    users: UserStats,
    s3: S3Stats,
    photos: PhotoStats,
}

pub fn fetch_dashboard(s3: &S3Access, db: &PgConnection) -> Result<AdminContext, ApiError> {
    let users = ApiError::server_error(count_users(db))?;
    let s3 = s3_stats(s3);
    let photos = ApiError::server_error(count_photos(db))?;
    Ok(AdminContext { users, s3, photos })
}

#[derive(Serialize)]
struct S3Stats {
    bucket: String,
    cdn: String,
    cdn_prefix: Option<String>,
}

fn s3_stats(s3: &S3Access) -> S3Stats {
    S3Stats {
        bucket: s3.bucket.clone(),
        cdn: s3.cdn_url.clone(),
        cdn_prefix: s3.cdn_prefix.clone(),
    }
}

#[derive(Serialize)]
struct UserStats {
    total: i64,
    subscribed: i64,
}

fn count_users(db: &PgConnection) -> Result<UserStats, Error> {
    use db::schema::users::dsl::*;
    let total = users.select(count(id)).first(db)?;
    let subscribed = users.select(count(subscription_expires)).first(db)?;
    Ok(UserStats { total, subscribed })
}

#[derive(Serialize)]
struct PhotoStats {
    created: i64,
    uploaded: i64,
}

fn count_photos(db: &PgConnection) -> Result<PhotoStats, Error> {
    use db::schema::photos::dsl::*;
    let created = photos.select(count(id)).first(db)?;
    let uploaded = photos.select(count(present)).first(db)?;
    Ok(PhotoStats { created, uploaded })
}
