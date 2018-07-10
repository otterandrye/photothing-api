#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate rocket_cors;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate futures;

mod s3;

use rocket::fairing::AdHoc;
use rocket::State;
use rocket::http::Method;
use rocket_contrib::Json;
use rocket_cors::{AllowedOrigins, AllowedHeaders};
use s3::{S3Access, UploadRequest, UploadResponse};

#[get("/")]
fn index() -> &'static str {
    "Hello, rocket!"
}

#[post("/upload", data = "<req>")]
fn sign_upload(s3: State<S3Access>, req: Json<UploadRequest>) -> Json<UploadResponse> {
    Json(s3::sign_upload(s3.inner(), "TODO", req.into_inner()))
}

pub fn rocket() -> rocket::Rocket {
    let cors = rocket_cors::Cors {
        allowed_origins: AllowedOrigins::all(), // TODO: put into configuration
        allowed_methods: vec![Method::Get, Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    };
    rocket::ignite()
        .attach(AdHoc::on_attach(|rocket| {
            let bucket = rocket.config().get_str("s3_bucket_name")
                .expect("missing S3 bucket").to_owned();
            Ok(rocket.manage(S3Access::new(bucket)))
        }))
        .attach(cors)
        .mount("/", routes![index])
        .mount("/api", routes![sign_upload])
}

#[cfg(test)]
mod test {
    use super::rocket;
    use std::env;
    use rocket::local::Client;
    use rocket::http::{ContentType, Status};

    fn client() -> Client {
        env::set_var("ROCKET_S3_BUCKET_NAME", "foo");
        env::set_var("AWS_ACCESS_KEY_ID", "no");
        env::set_var("AWS_SECRET_ACCESS_KEY", "nope");
        Client::new(rocket()).expect("valid rocket instance")
    }

    #[test]
    fn hello_world() {
        let client = client();
        let mut response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some("Hello, rocket!".into()));
    }

    #[test]
    fn upload_missing_field() {
        let client = client();
        let response = client.post("/api/upload")
            .header(ContentType::JSON)
            .body(r#"{ "filename": "foo" }"#) // file_type missing
            .dispatch();
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[test]
    fn test_upload() {
        let client = client();
        let response = client.post("/api/upload")
            .header(ContentType::JSON)
            .body(r#"{ "filename": "foo", "file_type": "bar" }"#)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}
