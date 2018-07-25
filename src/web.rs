use dotenv;

use rocket::fairing::AdHoc;
use rocket::{ignite, Rocket, State};
use rocket::http::{Status, Cookies, Method};
use rocket::response::Failure;
use rocket::response::status::Custom;
use rocket_contrib::Json;
use rocket_cors::{Cors, AllowedOrigins, AllowedHeaders};

use db::{init_db_pool, DbConn};
use errors::ApiError;
use s3::{S3Access, UploadRequest};
use auth::{self, Subscriber, User, UserLogin, UserCreateResponse};
use photos;

#[post("/login", data = "<user>")]
fn login(db: DbConn, cookies: Cookies, user: Json<UserLogin>) -> Result<String, Custom<String>> {
    match auth::login_user(user.into_inner(), &db, cookies) {
        Some(user) => Ok(format!("Hello, {}", user.email)),
        None => Err(Custom(Status::Unauthorized, String::from("Username or password is invalid"))),
    }
}

#[post("/logout")]
fn logout(_user: User, cookies: Cookies) -> String {
    auth::logout(cookies);
    String::from(r#"{"logout":"Ok"}"#)
}

#[post("/logout", rank = 2)]
fn logout_no_user() -> Failure {
    Failure(Status::Unauthorized)
}

#[post("/register", data = "<user>")]
fn register(db: DbConn, user: Json<UserLogin>) -> Json<UserCreateResponse> {
    Json(auth::create_user(user.into_inner(), &db))
}

#[post("/upload", data = "<req>")]
fn sign_user_upload(user: Subscriber, s3: State<S3Access>, db: DbConn, req: Json<UploadRequest>)
    -> Result<Json<photos::PendingUpload>, ApiError>
{
    let user = user.0;
    let photo = photos::create_photo(&user, &db, s3.inner(), req.into_inner())?;
    Ok(Json(photo))
}

#[get("/photos")]
fn get_photos(user: User, db: DbConn) -> Result<Json<Vec<photos::Photo>>, ApiError> {
    let photos = photos::user_photos(&user, &db)?;
    Ok(Json(photos))
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
            let cdn_url = rocket.config().get_str("cdn_url")
                .expect("missing CDN url").to_owned();
            let cdn_prefix = rocket.config().get_str("cdn_prefix")
                .expect("missing CDN prefix").to_owned();
            let cdn_prefix = match cdn_prefix {
                _ if cdn_prefix.len() == 0 => None,
                _ => Some(cdn_prefix),
            };
            Ok(rocket.manage(S3Access::new(bucket, cdn_url, cdn_prefix)))
        }))
        .manage(init_db_pool())
        .attach(cors)
        .mount("/api", routes![
            login, logout, logout_no_user, register,
            sign_user_upload, get_photos
        ])
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
    fn logout_no_user() {
        let client = client();
        let response = client.post("/api/logout")
            .header(ContentType::JSON)
            .body("{}".to_string())
            .dispatch();
         assert_eq!(response.status(), Status::Unauthorized);
    }

    #[test]
    fn upload_no_user() {
        let client = client();
        let response = client.post("/api/upload")
            .header(ContentType::JSON)
            .body(format!("{}", json!({"filename": "foo", "file_type": "bar"})))
            .dispatch();
         assert_eq!(response.status(), Status::Unauthorized);
    }
}
