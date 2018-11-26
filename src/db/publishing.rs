use chrono::prelude::*;
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;
use diesel_derives::{belongs_to, Queryable, Identifiable, Associations};

use crate::db::schema::published_albums;
use crate::db::user::User;
use crate::db::album::Album;

#[derive(Queryable, Associations, Identifiable)]
#[belongs_to(User)]
#[belongs_to(Album)]
pub struct PublishedAlbum {
    pub id: i32,
    pub album_id: i32,
    pub user_id: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name="published_albums"]
struct NewPublish {
    user_id: i32,
    album_id: i32,
}

impl PublishedAlbum {
    pub fn publish(db: &PgConnection, album: Album) -> Result<Self, Error> {
        use db::schema::published_albums::dsl::*;
        let new = NewPublish {
            album_id: album.id,
            user_id: album.user_id,
        };
        let created = diesel::insert_into(published_albums).values(&new).get_result(db)?;
        Ok(created)
    }

    pub fn set_active(self, db: &PgConnection, new_active: bool) -> Result<Self, Error> {
        use db::schema::published_albums::dsl::*;
        diesel::update(&self)
            .set(active.eq(new_active))
            .get_result(db)
    }

    pub fn delete(self, db: &PgConnection) -> Result<(), Error> {
        diesel::delete(&self).execute(db)?;
        Ok(())
    }

    pub fn by_id(db: &PgConnection, given_id: i32) -> Result<Option<Self>, Error> {
        use db::schema::published_albums::dsl::*;
        published_albums
            .filter(id.eq(given_id))
            .first(db).optional()
    }

    pub fn for_user(db: &PgConnection, user: &User) -> Result<Vec<PublishedAlbum>, Error> {
        PublishedAlbum::belonging_to(user).load(db)
    }
}

#[cfg(test)]
impl PublishedAlbum {
    pub fn fake() -> Self {
        PublishedAlbum {
            id: 22,
            user_id: 3,
            album_id: 390,
            active: true,
            created_at: Utc::now(),
        }
    }
}
