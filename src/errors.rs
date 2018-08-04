use std::fmt::Debug;
use std::io::Cursor;

use rocket::{Response, Request};
use rocket::http::{ContentType, Status};
use rocket::response::{Result as RocketResult, Responder};

#[derive(Debug)]
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

impl ApiError {
    fn message_with_status<T, E: Debug>(e: Result<T, E>, status: Status) -> Result<T, ApiError> {
        e.map_err(|e|
            ApiError {
                status,
                message: format!("{:?}", e),
            })
    }

    pub fn server_error<T, E: Debug>(e: Result<T, E>) -> Result<T, ApiError> {
        {
            let _log = e.as_ref().map_err(|e| error!("Caught error: {:?}", e));
        }
        ApiError::message_with_status(e, Status::BadRequest)
    }

    pub fn bad_request<T, E: Debug>(e: Result<T, E>) -> Result<T, ApiError> {
        ApiError::message_with_status(e, Status::BadRequest)
    }

    pub fn unauthorized() -> ApiError {
        ApiError {
            status: Status::Unauthorized,
            message: "Username or password is invalid".to_string(),
        }
    }
}
