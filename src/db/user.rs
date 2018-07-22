use chrono::{NaiveDate, NaiveDateTime};
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;

use db::schema::users;
use ::util::{HashedPassword, uuid};

#[derive(Queryable, Identifiable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub uuid: String,
    pub password: String,
    pub name: Option<String>,
    pub subscription_expires: Option<NaiveDate>,
    pub updated_at: NaiveDateTime,
}

impl User {
    // Look up a user by email. Returns error in the case of db failures so we can distinguish
    // 400 and 500 cases
    pub fn by_email(db: &PgConnection, user_email: &str) -> Result<Option<User>, Error> {
        use db::schema::users::dsl::*;
        let mut results = users.filter(email.eq(user_email))
            .limit(1)
            .load::<User>(db)?;
        if results.get(0).is_none() {
            Ok(None)
        } else {
            Ok(Some(results.swap_remove(0)))
        }
    }
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser {
    email: String,
    uuid: String,
    password: String,
    name: Option<String>,
    subscription_expires: Option<NaiveDate>,
}

impl NewUser {
    pub fn new(email: String, pw: HashedPassword) -> NewUser {
        NewUser {
            email,
            uuid: uuid().0,
            password: pw.0,
            name: None,
            subscription_expires: None,
        }
    }

    pub fn insert(self, db: &PgConnection) -> Result<User, Error> {
        use db::schema::users::dsl::*;
        let created = diesel::insert_into(users).values(&self).get_result(db)?;
        Ok(created)
    }
}
