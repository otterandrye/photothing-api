use chrono::prelude::*;
use chrono::{Duration, DateTime, NaiveDate};
use diesel;
use diesel::PgConnection;
use diesel::prelude::*;

use db::schema::{users, password_resets};
use ::util::{HashedPassword, uuid};

#[derive(Queryable, Identifiable, Clone)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub uuid: String,
    pub password: String,
    pub name: Option<String>,
    pub subscription_expires: Option<NaiveDate>,
    pub updated_at: DateTime<Utc>,
    pub joined: DateTime<Utc>,
}

impl User {
    // Look up a user by email. Returns error in the case of db failures so we can distinguish
    // 400 and 500 cases
    pub fn by_email(db: &PgConnection, user_email: &str) -> QueryResult<Option<User>> {
        use db::schema::users::dsl::*;
        users.filter(email.eq(user_email)).first(db).optional()
    }

    pub fn by_id(db: &PgConnection, user_id: i32) -> QueryResult<Option<User>> {
        use db::schema::users::dsl::*;
        users.filter(id.eq(user_id)).first(db).optional()
    }

    // Select a user row for update (locks the row)
    pub fn for_update(db: &PgConnection, user_email: &str) -> QueryResult<Option<MutableUser>> {
        use db::schema::users::dsl::*;
        match users.for_update().filter(email.eq(user_email)).first(db).optional() {
            Ok(Some(user)) => Ok(Some(MutableUser(user))),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// Wrapper to prevent modifying users without locking the row first
pub struct MutableUser(User);

impl MutableUser {
    pub fn edit_subscription(self, db: &PgConnection, expiration_date: Option<NaiveDate>) -> QueryResult<User> {
        use db::schema::users::dsl::*;
        diesel::update(&self.0)
            .set(subscription_expires.eq(expiration_date))
            .get_result(db)
    }

    pub fn change_password(self, db: &PgConnection, pw: HashedPassword) -> QueryResult<User> {
        use db::schema::users::dsl::*;
        diesel::update(&self.0)
            .set(password.eq(pw.0))
            .get_result(db)
    }
}

#[cfg(test)]
impl User {
    pub fn fake() -> User {
        let now = Utc::now();
        User {
            id: 1, email: String::from("foo"),
            uuid: uuid().0, password: String::from("nope"),
            name: None, subscription_expires: None,
            updated_at: now,
            joined: now,
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

    pub fn insert(self, db: &PgConnection) -> QueryResult<User> {
        use db::schema::users::dsl::*;
        let created = diesel::insert_into(users).values(&self).get_result(db)?;
        Ok(created)
    }
}

#[derive(Queryable, Identifiable, Associations, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[primary_key(uuid)]
#[belongs_to(User, foreign_key = "user_id")]
pub struct PasswordReset {
    pub uuid: String,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
}

impl PasswordReset {
    pub fn by_uuid(db: &PgConnection, user: &MutableUser, given_uuid: &str) -> QueryResult<Option<PasswordReset>> {
        use db::schema::password_resets::dsl::*;
        let reset = PasswordReset::belonging_to(&user.0)
            .for_update() // always lock the pw reset row
            .filter(uuid.eq(given_uuid))
            .first::<PasswordReset>(db)
            .optional()?;
        if let Some(reset) = reset {
            let now = Utc::now();
            if reset.created_at.signed_duration_since(now) < Duration::hours(24) {
                return Ok(Some(reset))
            } else {
                reset.delete(db)?;
            }
        }
        Ok(None)
    }

    pub fn create(user: &User, db: &PgConnection) -> QueryResult<PasswordReset> {
        use db::schema::password_resets::dsl::*;
        let reset = NewPasswordReset { user_id: user.id, uuid: ::util::uuid().0 };
        let created = diesel::insert_into(password_resets)
            .values(&reset)
            .get_result(db)?;
        Ok(created)
    }

    pub fn delete(self, db: &PgConnection) -> QueryResult<usize> {
        diesel::delete(&self).execute(db)
    }
}

#[derive(Insertable)]
#[table_name = "password_resets"]
struct NewPasswordReset {
    user_id: i32,
    uuid: String,
}

#[cfg(test)]
impl NewUser {
    pub fn fake(email: &str) -> Self {
        NewUser::new(String::from(email), HashedPassword(String::from("foobar")))
    }
}

#[cfg(test)]
impl PasswordReset {
    pub fn fake(uuid: &str) -> Self {
        PasswordReset {
            uuid: uuid.to_owned(),
            user_id: 1,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod functest {
    use db::test_db;
    use super::*;

    #[ignore]
    #[test]
    fn user_crud() {
        let db = test_db();
        let user = NewUser::fake("foo");
        let user = user.insert(&db).expect("couldn't make user");
        assert!(user.subscription_expires.is_none(), "new user has subscription by default");
        assert_eq!(user.email, "foo");

        let date = NaiveDate::from_ymd(2015, 3, 14);
        let mutable = User::for_update(&db, &user.email).expect("db").expect("found user");
        let updated = mutable.edit_subscription(&db, Some(date)).expect("update error'd");
        assert_eq!(Some(date), updated.subscription_expires, "date update failed");

        let mutable = User::for_update(&db, &user.email).expect("db").expect("found user");
        let back_to_null = mutable.edit_subscription(&db, None).expect("back to None failed");
        assert!(back_to_null.subscription_expires.is_none(), "date update failed");

        let mutable = User::for_update(&db, &user.email).expect("db").expect("found user");
        let new_pw = mutable.change_password(&db, HashedPassword(String::from("foo")))
            .expect("change pw failed");
        assert_eq!(new_pw.password, "foo");
    }

    #[ignore]
    #[test]
    fn password_reset_crud() {
        let db = test_db();
        let user = NewUser::fake("pw_crud").insert(&db).expect("couldn't make user");
        let m_user = User::for_update(&db, &user.email)
            .expect("couldnt get for update").expect("found user");

        let user2 = NewUser::fake("pw_crud2").insert(&db).expect("couldn't make 2nd user");
        let m_user2 = User::for_update(&db, &user2.email)
            .expect("couldnt get for update").expect("found user");

        let reset = PasswordReset::create(&user, &db).expect("create failed");
        assert_eq!(reset.user_id, user.id);

        let from_db = PasswordReset::by_uuid(&db, &m_user, &reset.uuid);
        assert_eq!(from_db, Ok(Some(reset.clone())));

        // make sure the query checks the user id
        let wrong_user = PasswordReset::by_uuid(&db, &m_user2, &reset.uuid);
        assert_eq!(wrong_user, Ok(None));

        let deleted = from_db.unwrap().unwrap().delete(&db);
        assert_eq!(deleted, Ok(1));

        // can't fetch once it's deleted
        let from_db = PasswordReset::by_uuid(&db, &m_user, &reset.uuid);
        assert_eq!(from_db, Ok(None));
    }
}
