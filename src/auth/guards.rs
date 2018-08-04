use chrono::prelude::*;
use rocket::Outcome;
use rocket::http::{Cookie, Status};
use rocket::request::{self, Request, FromRequest};

pub use db::user::User;

use super::{logout, USER_COOKIE, AUTH_HEADER, ADMIN_PREFIX};
use db::DbConn;

// Check the private cookies or custom auth header on the request to see if there's a stored user
// id. If there is, look up the user to make sure the user is still valid in the database. This
// will handle authentication for all requests with a [`User`] guard
// TODO: cache this somehow to reduce 1 x db call per request
impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<User, ()> {
        let user = User::check_headers(request)
            .or_else(|| User::check_cookies(request));

        match user {
            Some(user_email) => {
                let db = request.guard::<DbConn>()?;
                match User::by_email(&db, &user_email) {
                    Ok(Some(user)) => Outcome::Success(user),
                    Ok(None) => {
                        // the user has been removed, clear the invalid cookie
                        logout(request.cookies());
                        Outcome::Failure((Status::Unauthorized, ()))
                    },
                    Err(e) => {
                        error!("Error fetching user from email ({}): {}", user_email, e);
                        Outcome::Failure((Status::InternalServerError, ()))
                    },
                }
            }
            None => Outcome::Failure((Status::Unauthorized, ())),
        }
    }
}

impl User {
    fn check_cookies(request: &Request) -> Option<String> {
        request.cookies()
            .get_private(USER_COOKIE)
            .map(|cookie| cookie.value().to_string())
    }

    fn check_headers(request: &Request) -> Option<String> {
        // This is a bit of a hack: the encryption rocket uses is buried inside its
        // cookie jar, so if we see an auth header we stick it into the request as a cookie
        // so the normal mechanism can decrypt it for us. Hence 'add' instead of 'add_private'
        if let Some(encrypted) = request.headers().get_one(AUTH_HEADER) {
            request.cookies().add(Cookie::new(USER_COOKIE, encrypted.to_string()));
        }
        None
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
