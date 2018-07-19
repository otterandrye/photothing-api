use chrono::NaiveDateTime;
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;

use db::schema::photos;
use db::user::User;
use ::util::uuid;

#[derive(Queryable)]
pub struct Photo {
    pub id: i32,
    pub uuid: String,
    pub owner: i32, // users.id
    pub present: Option<bool>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name="photos"]
pub struct NewPhoto {
    uuid: String,
    owner: i32,
}

impl NewPhoto {
    pub fn new(owner: User) -> NewPhoto {
        NewPhoto {
            owner: owner.id,
            uuid: uuid().0,
        }
    }

    pub fn insert(self, db: &PgConnection) -> Result<Photo, Error> {
        use db::schema::photos::dsl::*;
        let created = diesel::insert_into(photos).values(&self).get_result(db)?;
        Ok(created)
    }
}
