use bcrypt::verify;
use mailchecker;
use rocket::Outcome;
use rocket::http::{Cookie, Cookies, Status};
use rocket::request::{self, Request, FromRequest};
use chrono::prelude::*;
use zxcvbn::zxcvbn as check_password;

use db::DbConn;
use db::user::NewUser;
use errors::ApiError;
use util::hash_password;

pub use db::user::User;

static USER_COOKIE: &str = "u";
static ADMIN_PREFIX: &str = "ADMINx";

// These errors are API-facing, return tokens rather than english
static PW_SHORT_ERROR: &str = "PW_TOO_SHORT_8_MIN";
static PW_SIMPLE: &str = "PW_TOO_SIMPLE";
static EMAIL_ERROR: &str = "INVALID_EMAIL";

#[derive(Deserialize, Debug)]
pub struct UserLogin {
    email: String,
    password: String,
}

impl UserLogin {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.password.len() < 8 {
            return Err(PW_SHORT_ERROR);
        } else if let Ok(entropy) = check_password(&self.password, &vec![&self.email[..]][..]) {
            if entropy.score < 3 {
                return Err(PW_SIMPLE);
            }
        }
        if !mailchecker::is_valid(&self.email) {
            return Err(EMAIL_ERROR);
        }
        Ok(())
    }
}

#[derive(Serialize, Debug)]
pub struct UserCreateResponse {
    email: String,
}

// Create a new user account and set the login cookie
pub fn create_user(new_user: UserLogin, db: &DbConn) -> Result<UserCreateResponse, ApiError> {
    let email = new_user.email.clone();

    ApiError::bad_request(new_user.validate())?;
    let hashed = ApiError::bad_request(hash_password(&new_user.password))?;

    // Validation and password hashing completed successfully, insert the new user
    let user = NewUser::new(email.clone(), hashed);

    // Always say "registration accepted, please log in now" regardless of status
    // e.g. if there's an error because an email address is already in use don't tell the user
    match user.insert(db) {
        Ok(user) => Ok(UserCreateResponse { email: user.email }),
        Err(e) => {
            error!("Error inserting new user ({}): {}", email, e);
            Ok(UserCreateResponse { email })
        }
    }
}

// Check the provided email/password against the database
//  - Set a private cookie on success
//  - don't send the user any database errors, could leak sensitive info
#[allow(unused_must_use)]
pub fn login_user(creds: UserLogin, db: &DbConn, cookies: Cookies) -> Option<User> {
    match User::by_email(db, &creds.email) {
        Err(e) => {
            error!("Error fetching user ({:?}): {}", creds, e);
            None
        },
        Ok(Some(user)) => {
            match verify(&creds.password, &user.password) {
                Ok(true) => {
                    login(cookies, &user);
                    Some(user)
                },
                _ => None,
            }

        },
        _ => {
            // Run verify in the "email not registered" case too to prevent timing attacks
            // Ok that we ignore the Result
            verify("run verify here so attackers", "can't use timing attacks against login");
            None
        },
    }
}

pub fn logout(mut cookies: Cookies) {
    cookies.remove_private(Cookie::named(USER_COOKIE));
}

fn login(mut cookies: Cookies, user: &User) {
    let login_cookie = Cookie::build(USER_COOKIE, user.email.clone())
        .secure(false) // appears to be required to make CORS work
        .http_only(true)
        .finish();
    cookies.add_private(login_cookie);
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

// Use the first characters of 'user.uuid' to identify admins
// This should work until we decide we want more fine-grained user access levels
pub struct Admin(User);

impl<'a, 'r> FromRequest<'a, 'r> for Admin {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Admin, ()> {
        let user = request.guard::<User>()?;
        Admin::from_user(user)
    }
}

impl Admin {
    fn from_user(user: User) -> request::Outcome<Admin, ()> {
        if user.uuid.starts_with(ADMIN_PREFIX) {
            return Outcome::Success(Admin(user));
        }
        // 404 for any admin-specific URLs if auth fails
        Outcome::Failure((Status::NotFound, ()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn user_form_validation() {
        let short_pw = UserLogin { email: String::from("a@g.com"), password: String::from("hi") };
        assert_eq!(short_pw.validate(), Err(PW_SHORT_ERROR));

        let simple_pw = UserLogin { email: String::from("a@g.com"), password: String::from("12345678") };
        assert_eq!(simple_pw.validate(), Err(PW_SIMPLE));

        let bad_email = UserLogin { email: String::from("not an email"), password: String::from("minimum eight chars") };
        assert_eq!(bad_email.validate(), Err(EMAIL_ERROR));

        let ascii_pw = UserLogin { email: String::from("foo@gmail.com"), password: String::from("hàµyKµ3øã^^³½ä}A5öý9×¿aûiëP}·") };
        assert!(ascii_pw.validate().is_ok());
    }

    #[test]
    fn subscription_check() {
        let mut user = User::fake();

        let null_column = Subscriber::from_user(user.clone());
        assert!(null_column.is_failure());

        let long_ago = NaiveDate::from_ymd(2015, 3, 14);
        user.subscription_expires = Some(long_ago);
        let expired = Subscriber::from_user(user.clone());
        assert!(expired.is_failure());

        let far_from_now = NaiveDate::from_ymd(2200, 3, 14);
        user.subscription_expires = Some(far_from_now);
        let unexpired = Subscriber::from_user(user.clone());
        assert!(unexpired.is_success());
    }

    #[test]
    fn admin_check() {
        let mut user = User::fake();

        let not_admin = Admin::from_user(user.clone());
        assert!(not_admin.is_failure());

        let admin_uuid = format!("{}{}", ADMIN_PREFIX, user.uuid);
        user.uuid = admin_uuid[..32].to_string();

        let admin = Admin::from_user(user.clone());
        assert!(admin.is_success());
    }
}

#[cfg(test)]
mod functest {
    use dotenv;
    use db::{DbConn, init_db_pool};
    use super::*;

    #[test]
    fn edit_subscription() {
        dotenv::dotenv().ok();
        let pool = init_db_pool();
        let db = DbConn(pool.get().expect("couldn't connect to db"));
        let email = "subs";
        let user = NewUser::fake(email);
        let user = user.insert(&db).expect("couldn't make user");

        let long_ago = NaiveDate::from_ymd(2015, 3, 14);
        let user = user.edit_subscription(&db, Some(long_ago)).expect("edit failed");
        assert_eq!(user.subscription_expires, Some(long_ago));

        let user = user.edit_subscription(&db, None).expect("edit failed");
        assert_eq!(user.subscription_expires, None);
    }
}
