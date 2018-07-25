use bcrypt::verify;
use mailchecker;
use rocket::Outcome;
use rocket::http::{Cookie, Cookies, Status};
use rocket::request::{self, Request, FromRequest};
use chrono::prelude::*;

use db::DbConn;
use db::user::NewUser;
use util::hash_password;

pub use db::user::User;

static USER_COOKIE: &str = "u";

// These errors are API-facing, return tokens rather than english
static PW_LENGTH_ERROR: &str = "PW_TOO_LONG_70_MAX";
static PW_SHORT_ERROR: &str = "PW_TOO_SHORT_8_MIN";
static EMAIL_ERROR: &str = "INVALID_EMAIL";

#[derive(Deserialize, Debug)]
pub struct UserLogin {
    email: String,
    password: String,
}

impl UserLogin {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.password.len() >= 70 {
            return Err(PW_LENGTH_ERROR);
        } else if self.password.len() < 8 {
            return Err(PW_SHORT_ERROR);
        } else if !mailchecker::is_valid(&self.email) {
            return Err(EMAIL_ERROR);
        }
        Ok(())
    }
}

#[derive(Serialize, Debug)]
pub struct UserCreateResponse {
    email: String,
    error: Option<String>,
}

// Create a new user account and set the login cookie
pub fn create_user(new_user: UserLogin, db: &DbConn) -> UserCreateResponse {
    let email = new_user.email.clone();

    if let Err(e) = new_user.validate() {
        return UserCreateResponse { email, error: Some(e.to_string()) };
    }
    let hashed = hash_password(&new_user.password);
    if let Err(e) = hashed {
        return UserCreateResponse { email, error: Some(e) };
    }
    // Validation and password hashing completed successfully, insert the new user
    let user = NewUser::new(email.clone(), hashed.unwrap());

    // Always say "registration accepted, please log in now" regardless of status
    // e.g. if there's an error because an email address is already in use don't tell the user
    match user.insert(db) {
        Ok(user) => {
            UserCreateResponse { email: user.email, error: None }
        },
        Err(e) => {
            error!("Error inserting new user ({}): {}", email, e);
            UserCreateResponse { email, error: None }
        }
    }
}

// Check the provided email/password against the database
//  - Set a private cookie on success
//  - don't send the user any database errors, could leak sensitive info
pub fn login_user(creds: UserLogin, db: &DbConn, mut cookies: Cookies) -> Option<User> {
    match User::by_email(db, &creds.email) {
        Err(e) => {
            error!("Error fetching user ({:?}): {}", creds, e);
            None
        },
        Ok(Some(user)) => {
            match verify(&creds.password, &user.password) {
                Ok(true) => {
                    cookies.add_private(Cookie::new(USER_COOKIE, format!("{}", user.email)));
                    Some(user)
                },
                _ => None,
            }

        },
        _ => None,
    }
}

pub fn logout(mut cookies: Cookies) {
    cookies.remove_private(Cookie::named(USER_COOKIE));
}

// Check the private cookies on the request to see if there's a stored user id. If there is,
// look up the user to make sure the user is still valid in the database. This will handle
// authentication for all requests with a `User` guard
impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<User, ()> {
        let user_cookie = request.cookies().get_private(USER_COOKIE);
        match user_cookie {
            Some(user_email) => {
                let db = request.guard::<DbConn>()?;
                match User::by_email(&db, user_email.value()) {
                    Ok(Some(user)) => Outcome::Success(user),
                    Ok(None) => {
                        // the user has been removed, clear the invalid cookie
                        logout(request.cookies());
                        Outcome::Failure((Status::Unauthorized, ()))
                    },
                    Err(e) => {
                        error!("Error fetching user from cookie ({}): {}", user_email, e);
                        Outcome::Failure((Status::InternalServerError, ()))
                    },
                }
            }
            None => Outcome::Failure((Status::Unauthorized, ())),
        }
    }
}

// A subscriber is a user with a non-null subscription expiry date that is after today
pub struct Subscriber(pub User);

impl<'a, 'r> FromRequest<'a, 'r> for Subscriber {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Subscriber, ()> {
        let user = request.guard::<User>()?;
        Subscriber::from_user(user)
    }
}

impl Subscriber {
    fn from_user(user: User) -> request::Outcome<Subscriber, ()> {
        let utc: DateTime<Utc> = Utc::now();
        let today = utc.num_days_from_ce();
        match user.subscription_expires {
            Some(expiration) if expiration.num_days_from_ce() >= today =>
                Outcome::Success(Subscriber(user)),
            _ => Outcome::Failure((Status::Forbidden, ())),
        }
    }
}

#[cfg(test)]
mod test {
    use dotenv;

    use db::{DbConn, init_db_pool};
    use super::*;

    #[test]
    fn user_form_validation() {
        let part = String::from("abcdefghijklmnopqrstuvwyzx");
        let pw = format!("{}{}{}{}{}{}{}", part, part, part, part, part, part, part);
        assert!(pw.len() > 70);
        let long_pw = UserLogin { email: String::from("a@g.com"), password: pw };
        assert_eq!(long_pw.validate(), Err(PW_LENGTH_ERROR));

        let short_pw = UserLogin { email: String::from("a@g.com"), password: String::from("hi") };
        assert_eq!(short_pw.validate(), Err(PW_SHORT_ERROR));

        let bad_email = UserLogin { email: String::from("not an email"), password: String::from("minimum eight chars") };
        assert_eq!(bad_email.validate(), Err(EMAIL_ERROR));
    }

    #[test]
    fn subscription_check() {
        dotenv::dotenv().ok();
        let pool = init_db_pool();
        let db = DbConn(pool.get().expect("couldn't connect to db"));
        let email = "subs";
        let user = NewUser::fake(email);
        let user = user.insert(&db).expect("couldn't make user");

        let null_column = Subscriber::from_user(user.clone());
        assert!(null_column.is_failure());

        let long_ago = NaiveDate::from_ymd(2015, 3, 14);
        let user = user.edit_subscription(&db, Some(long_ago)).expect("edit failed");
        let expired = Subscriber::from_user(user.clone());
        assert!(expired.is_failure());

        let far_from_now = NaiveDate::from_ymd(2200, 3, 14);
        let user = user.edit_subscription(&db, Some(far_from_now)).expect("edit failed");
        let unexpired = Subscriber::from_user(user.clone());
        assert!(unexpired.is_success());
    }
}
