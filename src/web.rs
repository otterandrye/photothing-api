use dotenv;

use rocket::fairing::AdHoc;
use rocket::{ignite, Rocket, State};
use rocket::http::{Cookies, Method};
use rocket_contrib::Json;
use rocket_cors::{Cors, AllowedOrigins, AllowedHeaders};

use db::{init_db_pool, DbConn};
use db::models::{User, NewUser};
use s3::{sign_upload, S3Access, UploadRequest, UploadResponse};
use auth::{self, UserLogin, UserCreateResponse};

#[post("/login", data = "<user>")]
fn login(db: DbConn, cookies: Cookies, user: Json<UserLogin>) -> String {
    match auth::login_user(user.into_inner(), &db, cookies) {
        Some(user) => format!("Hello, {}", user.email),
        None => String::from("Username or password is invalid"),
    }
}

#[post("/logout")]
fn logout(_user: User, cookies: Cookies) -> String {
    auth::logout(cookies);
    String::from("Ok")
}

#[post("/register", data = "<user>")]
fn register(db: DbConn, cookies: Cookies, user: Json<NewUser>) -> Json<UserCreateResponse> {
    Json(auth::create_user(user.into_inner(), &db, cookies))
}

#[post("/upload", data = "<req>")]
fn sign_user_upload(user: User, s3: State<S3Access>, req: Json<UploadRequest>) -> Json<UploadResponse> {
    Json(sign_upload(s3.inner(), &user.uuid, req.into_inner()))
}

// Main entry that creates the web application, connects to the database and binds the web routes
pub fn rocket() -> Rocket {
    dotenv::dotenv().ok(); // read from a .env file if one is present
    let cors = Cors {
        allowed_origins: AllowedOrigins::all(), // TODO: put into configuration
        allowed_methods: vec![Method::Get, Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    };
    ignite()
        .attach(AdHoc::on_attach(|rocket| {
            let bucket = rocket.config().get_str("s3_bucket_name")
                .expect("missing S3 bucket").to_owned();
            Ok(rocket.manage(S3Access::new(bucket)))
        }))
        .manage(init_db_pool())
        .attach(cors)
        .mount("/api", routes![login, register, sign_user_upload])
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::local::Client;
    use rocket::http::{ContentType, Status};

    fn client() -> Client {
        Client::new(rocket()).expect("valid rocket instance")
    }

    #[test]
    fn upload_endpoint_missing_field() {
        let client = client();
        let response = client.post("/api/upload")
            .header(ContentType::JSON)
            .body(r#"{ "filename": "foo" }"#) // file_type missing
            .dispatch();
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[test]
    fn upload_endpoint() {
        let client = client();
        let response = client.post("/api/upload")
            .header(ContentType::JSON)
            .body(r#"{ "filename": "foo", "file_type": "bar" }"#)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}
