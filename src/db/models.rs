use chrono::NaiveDate;
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;
use mailchecker;

use db::schema::users;

#[derive(Queryable, Insertable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub uuid: String,
    pub password: String,
    pub name: Option<String>,
    pub subscription_expires: Option<NaiveDate>,
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

#[derive(Insertable, Deserialize, Debug)]
#[table_name="users"]
pub struct NewUser {
    // TODO: stronger types here, hoist the validation
    pub email: String,
    password: String,
    name: Option<String>,
}

static PW_LENGTH_ERROR: &str = "Passwords must be 70 characters or less";
static EMAIL_ERROR: &str = "The provided email address is invalid";

impl NewUser {
    fn validate(&self) -> Result<(), &'static str> {
        if self.password.len() >= 70 {
            return Err(PW_LENGTH_ERROR);
        } else if !mailchecker::is_valid(&self.email) {
            return Err(EMAIL_ERROR);
        }
        Ok(())
    }

    pub fn insert(self, db: &PgConnection) -> Result<User, Error> {
        use db::schema::users::dsl::*;
        self.validate();
        let created = diesel::insert_into(users).values(&self).get_result(db)?;
        Ok(created)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn user_form_validation() {
        let part = String::from("abcdefghijklmnopqrstuvwyzx");
        let pw = format!("{}{}{}{}{}{}{}", part, part, part, part, part, part, part);
        assert!(pw.len() > 70);
        let long_pw = NewUser { email: String::from("a@g.com"), password: pw, name: None };
        assert_eq!(long_pw.validate(), Err(PW_LENGTH_ERROR));

        let bad_email = NewUser { email: String::from("not an email"), password: String::from("pw"), name: None};
        assert_eq!(bad_email.validate(), Err(EMAIL_ERROR));
    }
}
