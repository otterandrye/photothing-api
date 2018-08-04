use bcrypt::verify;
use mailchecker;
use rocket::http::{Cookie, Cookies};

use zxcvbn::zxcvbn as check_password;

use db::DbConn;
use db::user::{User, NewUser, PasswordReset};
use email::Emailer;
use errors::ApiError;
use util::hash_password;

pub mod guards;

static USER_COOKIE: &str = "u";
static AUTH_HEADER: &str = "X-Pt-Auth";
static ADMIN_PREFIX: &str = "ADMINx";

// These errors are API-facing, return tokens rather than english
static PW_SHORT_ERROR: &str = "PW_TOO_SHORT_8_MIN";
static PW_SIMPLE: &str = "PW_TOO_SIMPLE";
static EMAIL_ERROR: &str = "INVALID_EMAIL";

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
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

// Create a new user account
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

#[derive(Serialize)]
pub struct UserCredentials {
    email: String,
    header: String,
    pt_auth: String,
}

impl UserCredentials {
    fn new(user: User, auth_token: String) -> UserCredentials {
        UserCredentials {
            email: user.email,
            header: String::from(AUTH_HEADER),
            pt_auth: auth_token,
        }
    }
}

// Check the provided email/password against the database
//  - Set a private cookie on success
//  - don't send the user any database errors, could leak sensitive info
pub fn try_login_user(creds: UserLogin, db: &DbConn, cookies: Cookies) -> Option<UserCredentials> {
    match User::by_email(db, &creds.email) {
        Err(e) => {
            error!("Error fetching user ({:?}): {}", creds, e);
            None
        },
        Ok(Some(user)) => {
            match verify(&creds.password, &user.password) {
                Ok(true) => Some(login(cookies, user)),
                _ => None,
            }
        },
        _ => {
            // Run verify in the "email not registered" case too to prevent timing attacks
            let _ = verify("run verify here so attackers", "can't use timing attacks against login");
            None
        },
    }
}

pub fn logout(mut cookies: Cookies) {
    cookies.remove_private(Cookie::named(USER_COOKIE));
}

fn login(mut cookies: Cookies, user: User) -> UserCredentials {
    let login_cookie = Cookie::build(USER_COOKIE, user.email.clone())
        .secure(false) // appears to be required to make CORS work
        .http_only(true)
        .finish();
    cookies.add_private(login_cookie);
    let encrypted_cookie = cookies.get(USER_COOKIE).expect("added above");
    UserCredentials::new(user, encrypted_cookie.value().to_string())
}

pub fn start_password_reset(
    email: &str, db: &DbConn, emailer: &Emailer
) -> Result<Option<PasswordReset>, ApiError> {
    let user = ApiError::server_error(User::by_email(&db, &email))?;
    match user {
        Some(user) => {
            let reset = ApiError::server_error(PasswordReset::create(&user, db))?;
            // TODO: real message, handle failure via transaction
            let message = format!("Your password reset token is '{}'", &reset.uuid);
            {
                let mut client = ApiError::server_error(emailer.client.lock())?;
                ApiError::server_error(client.send_message(&user.email, &message))?;
            }
            Ok(Some(reset))
        }
        _ => Ok(None)
    }
}

pub fn handle_password_reset(reset: UserLogin, uuid: &str, db: &DbConn) -> Result<bool, ApiError> {
    // handle potential user-caused errors first, always hash to prevent timing attacks
    ApiError::bad_request(reset.validate())?;
    let hashed = ApiError::bad_request(hash_password(&reset.password))?;

    use diesel::Connection;
    use diesel::result::Error;

    // run this set of DB changes in a transaction so we never delete the reset w/o updating the pw
    ApiError::server_error::<_, Error>(db.transaction(|| {
        let user = User::for_update(&db, &reset.email)?;
        if let Some(user) = user {
            let reset_auth = PasswordReset::by_uuid(&db, &user, uuid)?;
            if let Some(reset_auth) = reset_auth {
                reset_auth.delete(&db)?;
                user.change_password(&db, hashed)?;
                return Ok(true)
            }
        }
        Ok(false) // deliberately don't tell the user why their request failed
    }))
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
}

#[cfg(test)]
mod functest {
    use chrono::Duration;
    use chrono::prelude::*;
    use db::test_db;
    use email::init_emailer;
    use super::*;

    #[test]
    fn password_reset_no_user() {
        let db = test_db();
        let mut emailer = init_emailer();
        let no_user = start_password_reset("foo@bizbang", &db, &mut emailer);
        assert_eq!(no_user, Ok(None));
        assert_eq!(emailer.client.try_lock().unwrap().messages().len(), 0);
    }

    #[test]
    fn password_reset() {
        let db = test_db();
        let mut emailer = init_emailer();
        let user = NewUser::fake("pw_reset_flow@gmail.com").insert(&db)
            .expect("couldn't make user");

        // kick off the password reset flow
        assert_eq!(emailer.client.try_lock().unwrap().messages().len(), 0);
        let reset = start_password_reset(&user.email, &db, &mut emailer)
            .expect("db error creating reset").expect("didn't get reset back");
        assert_eq!(reset.user_id, user.id);
        assert!(reset.created_at < Utc::now());
        assert!(reset.created_at.signed_duration_since(Utc::now()) < Duration::seconds(3));
        let client = emailer.client.try_lock().expect("locked emailer");
        let message = client.messages().get(0).expect("missing message");
        let expected = format!("<pw_reset_flow@gmail.com>::[Your password reset token is '{}']", &reset.uuid);
        assert_eq!(message, &expected);

        let new_pw = UserLogin {
            email: user.email.clone(),
            password: String::from("Gwc5C5KuavgeP5kBfhx7")
        };
        // can't reset without having the right magic token
        let reset_rejected = handle_password_reset(new_pw.clone(), "bad-reset-uuid", &db);
        assert_eq!(reset_rejected, Ok(false));

        // reset the user's password
        let reset_succeeded = handle_password_reset(new_pw.clone(), &reset.uuid, &db);
        assert_eq!(reset_succeeded, Ok(true));

        // verify that the DB was updated with the new password
        let user = User::by_email(&db, &new_pw.email).expect("db").expect("found");
        assert!(verify(&new_pw.password, &user.password).expect("bcrypt"));

        // and that we can't reset the PW again
        let reset_already_used = handle_password_reset(new_pw.clone(), &reset.uuid, &db);
        assert_eq!(reset_already_used, Ok(false));
    }
}
