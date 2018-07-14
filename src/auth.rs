use bcrypt::verify;
use mailchecker;
use rocket::Outcome;
use rocket::http::{Cookie, Cookies, Status};
use rocket::request::{self, Request, FromRequest};

use db::DbConn;
use db::models::{NewUser, User};
use util::hash_password;

static USER_COOKIE: &str = "u";

static PW_LENGTH_ERROR: &str = "Passwords must be 70 characters or less";
static EMAIL_ERROR: &str = "The provided email address is invalid";

#[derive(Deserialize, Debug)]
pub struct UserLogin {
    email: String,
    password: String,
}

impl UserLogin {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.password.len() >= 70 {
            return Err(PW_LENGTH_ERROR);
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
pub fn create_user(new_user: UserLogin, db: &DbConn, mut cookies: Cookies) -> UserCreateResponse {
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

    match user.insert(db) {
        Ok(user) => {
            cookies.add_private(Cookie::new(USER_COOKIE, format!("{}", user.email)));
            UserCreateResponse { email: user.email, error: None }
        },
        Err(e) => {
            error!("Error inserting new user ({}): {}", email, e);
            UserCreateResponse { email, error: Some(format!("{:?}", e)) }
        }
    }
}

// Check the provided email/password against the database
//  - Set a private cookie on success
//  - different returns for db and auth errors
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn user_form_validation() {
        let part = String::from("abcdefghijklmnopqrstuvwyzx");
        let pw = format!("{}{}{}{}{}{}{}", part, part, part, part, part, part, part);
        assert!(pw.len() > 70);
        let long_pw = UserLogin { email: String::from("a@g.com"), password: pw };
        assert_eq!(long_pw.validate(), Err(PW_LENGTH_ERROR));

        let bad_email = UserLogin { email: String::from("not an email"), password: String::from("pw") };
        assert_eq!(bad_email.validate(), Err(EMAIL_ERROR));
    }
}
