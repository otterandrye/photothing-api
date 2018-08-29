use std::fmt::Debug;
use std::io::Cursor;

use rocket::{Response, Request};
use rocket::http::{ContentType, Status};
use rocket::response::{Result as RocketResult, Responder};

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct ApiError {
    status: Status,
    message: String,
}

impl<'r> Responder<'r> for ApiError {
    fn respond_to(self, _: &Request) -> RocketResult<'r> {
        Response::build()
            .sized_body(Cursor::new(format!("{}", json!({"message": self.message}))))
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}

// These methods let us convert any error type into an `ApiError` so that web-facing methods
// can return a well-defined `Result` and use the `?` operator to check for failures
// for example:
// `ApiError::bad_request(new_user.validate())?;` to return a 400 response for invalid user input
impl ApiError {
    pub fn is_user_error(&self) -> bool {
        return self.status == Status::BadRequest
    }

    fn message_with_status<T, E: Debug>(e: Result<T, E>, status: Status) -> Result<T, ApiError> {
        e.map_err(|e| ApiError { status, message: format!("{:?}", e) })
    }

    pub fn server_error<T, E: Debug>(e: Result<T, E>) -> Result<T, ApiError> {
        {
            // These errors are always unexpected, so log an error message when they occur
            let _log = e.as_ref().map_err(|e| error!("Caught error: {:?}", e));
        }
        ApiError::message_with_status(e, Status::InternalServerError)
    }

    pub fn bad_request<T, E: Debug>(e: Result<T, E>) -> Result<T, ApiError> {
        ApiError::message_with_status(e, Status::BadRequest)
    }

    pub fn not_found<T>(opt: Option<T>, message: String) -> Result<T, ApiError> {
        match opt {
            Some(val) => Ok(val),
            None => Err(ApiError { status: Status::NotFound, message })
        }
    }

    pub fn unauthorized() -> ApiError {
        ApiError {
            status: Status::Unauthorized,
            message: "Username or password is invalid".to_string(),
        }
    }
}
