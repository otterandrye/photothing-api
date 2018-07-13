use bcrypt::{DEFAULT_COST, hash, verify, BcryptResult};
use rocket::Outcome;
use rocket::http::{Cookie, Cookies, Status};
use rocket::request::{self, Request, FromRequest};

use db::DbConn;
use db::models::{User, NewUser};

static USER_COOKIE: &str = "u";

#[derive(Deserialize, Debug)]
pub struct UserLogin {
    email: String,
    password: String,
}

#[derive(Serialize, Debug)]
pub struct UserCreateResponse {
    email: String,
    error: Option<String>,
}

// Create a new user account and set the login cookie
pub fn create_user(mut new_user: NewUser, db: &DbConn, mut cookies: Cookies) -> UserCreateResponse {
    let email = new_user.email.clone();

    // check validation here since it's annoying to from the insert function
    match new_user.validate() {
        Err(e) => return UserCreateResponse { email, error: Some(e.to_string()) },
        _ => {},
    }
    let hashed = hash_password(&new_user.password);
    match hashed {
        Ok(p) => new_user.password = p,
        Err(e) => return UserCreateResponse { email, error: Some(format!("Invalid password: {:?}", e)) },
    }

    match new_user.insert(db) {
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

fn hash_password(password: &str) -> BcryptResult<String> {
    hash(password, DEFAULT_COST)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn password_hashing() {
        // letters, numbers, special chars & extended ascii
        let pw = "åî>@%åÄSt»Æ·wj³´m~ðjC½µæGjq6?ï";
        let hashed = hash_password(pw).expect("hashing failed");

        assert!(verify(pw, &hashed).expect("hash failed"), "hashes match");
        assert!(!verify("moo moo", &hashed).expect("hash failed"), "diff strings dont match");
    }
}
