use std::io::Cursor;
use diesel::result::Error;
use rocket::{Response, Request};
use rocket::http::{ContentType, Status};
use rocket::response::{Result as RocketResult, Responder};

#[derive(Debug)]
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
    pub fn ise<T>(e: Result<T, Error>) -> Result<T, ApiError> {
        match e {
            Ok(t) => Ok(t),
            Err(dbe) => Err(ApiError {
                status: Status::InternalServerError,
                message: format!("{:?}", dbe),
            })
        }
    }
}