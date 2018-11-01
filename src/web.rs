use dotenv;
use harsh::{Harsh, HarshBuilder};
use rocket::fairing::AdHoc;
use rocket::{ignite, Rocket, State};
use rocket::http::{Cookies, Method};
use rocket_contrib::{Json, Template};
use rocket_cors::{Cors, AllowedOrigins, AllowedHeaders};
use serde_json::Value;

use db::{init_db_pool, DbConn, Pagination};
use email::{Emailer, init_emailer, dummy_emailer};
use errors::ApiError;
use s3::{S3Access, UploadRequest};
use auth;
use auth::guards::*;
use admin;
use albums;
use https;
use photos;

pub use db::Page;

pub type Api<T> = Result<Json<T>, ApiError>;
pub type FreeJson = Value;

#[post("/login", data = "<user>")]
fn login(
    db: DbConn, cookies: Cookies, user: Json<auth::UserLogin>
) -> Api<auth::UserCredentials> {
    match auth::try_login_user(user.into_inner(), &db, cookies) {
        Some(user) => Ok(Json(user)),
        None => Err(ApiError::unauthorized()),
    }
}

#[post("/logout")]
fn logout(_user: User, cookies: Cookies) -> Api<String> {
    auth::logout(cookies);
    Ok(Json(json!({"logout": "Ok"}).to_string()))
}

#[post("/register", data = "<user>")]
fn register(db: DbConn, user: Json<auth::UserLogin>) -> Api<auth::UserResponse> {
    let user = auth::create_user(user.into_inner(), &db)?;
    Ok(Json(user))
}

#[post("/reset_password/<email>")]
fn start_reset_password(
    db: DbConn, emailer: State<Emailer>, email: String,
) -> Api<String> {
    let status = match auth::start_password_reset(&email, &db, emailer.inner()) {
        Ok(_) => "Ok",
        Err(_) => "Failed",
    };
    Ok(Json(json!({"reset": status}).to_string()))
}

#[put("/reset_password/<uuid>", data="<user>")]
fn reset_password(db: DbConn, user: Json<auth::UserLogin>, uuid: String) -> Api<FreeJson> {
    match auth::handle_password_reset(user.into_inner(), &uuid, &db) {
        Err(ref e) if e.is_user_error() => Err(e.clone()),
        Ok(true) => Ok(Json(json!({"reset": "Ok"}))),
        _ => Ok(Json(json!({"reset": "Failed"}))),
    }
}

#[post("/upload", data = "<req>")]
fn sign_user_upload(
    user: Subscriber, s3: State<S3Access>, db: DbConn, req: Json<UploadRequest>
) -> Api<photos::PendingUpload> {
    let user = user.0;
    let photo = photos::create_photo(&user, &db, s3.inner(), req.into_inner())?;
    Ok(Json(photo))
}

#[get("/photos")]
fn get_photos(user: User, s3: State<S3Access>, db: DbConn) -> Api<Page<photos::Photo>> {
    get_photos_page(user, s3, db, Pagination::first())
}

#[get("/photos?<page>")]
fn get_photos_page(user: User, s3: State<S3Access>, db: DbConn, page: Pagination) -> Api<Page<photos::Photo>> {
    let photos = photos::user_photos(&user, &db, s3.inner(), page)?;
    Ok(Json(photos))
}

#[get("/albums?<page>")]
fn fetch_user_albums(user: User, db: DbConn, page: Pagination) -> Api<Page<albums::Album>> {
    let user_albums = albums::user_albums(&db, &user, page)?;
    Ok(Json(user_albums))
}

#[post("/albums?<details>")]
fn create_album(user: User, db: DbConn, details: albums::NewAlbum) -> Api<albums::Album> {
    let album = albums::create_album(&db, &user, details)?;
    Ok(Json(album))
}

#[get("/albums/<id>?<page>")]
fn fetch_album(user: User, s3: State<S3Access>, db: DbConn, id: i32, page: Pagination) -> Api<albums::Album> {
    let album = albums::fetch_album(&db, &user, s3.inner(), id, page)?;
    Ok(Json(album))
}

#[put("/albums/<id>/photos", data = "<photos>")]
fn add_photos_to_album(user: User, s3: State<S3Access>, db: DbConn, id: i32, photos: Json<Vec<i32>>) -> Api<albums::Album> {
    let album = albums::add_photos_to_album(&db, &user, s3.inner(), id, photos.into_inner())?;
    Ok(Json(album))
}

#[get("/albums/published")]
fn get_published_albums(user: User, db: DbConn, harsh: State<Harsh>) -> Api<Vec<albums::UrlFriendlyAlbum>> {
    let albums = albums::user_published_albums(&db, &user, harsh.inner())?;
    Ok(Json(albums))
}

#[post("/albums/<id>/publish")]
fn publish_album(user: User, db: DbConn, harsh: State<Harsh>, id: i32) -> Api<albums::UrlFriendlyAlbum> {
    let published = albums::publish_album(&db, &user, harsh.inner(), id)?;
    Ok(Json(published))
}

#[delete("/albums/<id>/photos", data = "<photos>")]
fn remove_photos_from_album(user: User, s3: State<S3Access>, db: DbConn, id: i32, photos: Json<Vec<i32>>) -> Api<albums::Album> {
    let album = albums::remove_photos_from_album(&db, &user, s3.inner(), id, photos.into_inner())?;
    Ok(Json(album))
}

#[get("/published/<hash_id>?<page>")]
fn get_published_photos(db: DbConn, s3: State<S3Access>, harsh: State<Harsh>, hash_id: String, page: Pagination) -> Api<albums::Album> {
    // Note: not an authenticated endpoint!
    let photos = albums::get_published_photos(&db, s3.inner(), harsh.inner(), hash_id, page)?;
    Ok(Json(photos))
}

#[delete("/published/<hash_id>")]
fn delete_published_album(user: User, db: DbConn, harsh: State<Harsh>, hash_id: String) -> Api<bool> {
    albums::delete_published_album(&db, &user, harsh.inner(), hash_id)?;
    Ok(Json(true))
}

#[post("/published/<hash_id>?<active>")]
fn toggle_published_album(db: DbConn, user: User, harsh: State<Harsh>, hash_id: String, active: albums::ToggleActive) -> Api<bool> {
    albums::toggle_published_album(&db, &user, harsh.inner(), hash_id, active)?;
    Ok(Json(true))
}

#[get("/admin")]
fn admin(_admin: Admin, s3: State<S3Access>, db: DbConn) -> Result<Template, ApiError> {
    let context = admin::fetch_dashboard(&s3.inner(), &db)?;
    Ok(Template::render("admin", &context))
}

#[get("/me")]
fn me(user: User) -> Api<auth::UserResponse> {
    Ok(Json(auth::UserResponse::new(user)))
}

// Main entry that creates the web application, connects to the database and binds the web routes
pub fn rocket() -> Rocket {
    dotenv::dotenv().ok(); // read from a .env file if one is present
    let cors = Cors {
        allowed_origins: AllowedOrigins::all(), // TODO: put into configuration
        allowed_methods: vec![Method::Get, Method::Post, Method::Put].into_iter().map(From::from).collect(),
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
        .attach(AdHoc::on_attach(|rocket| {
            let salt = rocket.config().get_str("id_salt").expect("missing salt").to_owned();
            let harsh = HarshBuilder::new().salt(salt).length(4)
                .init().expect("couldn't init id hasher");
            Ok(rocket.manage(harsh))
        }))
        .attach(AdHoc::on_attach(|rocket| {
            let email;
            {
                let key = rocket.config().get_str("mailgun_key");
                let domain = rocket.config().get_str("mailgun_domain");
                let app_url = rocket.config().get_str("app_url").expect("missing app url");

                email = match (key, domain) {
                    (Ok(key), Ok(domain))  => {
                        let system_email = format!("noreply@{}", &domain);
                        init_emailer(key, domain, &system_email, app_url)
                    },
                    _ => dummy_emailer(),
                };
            }
            Ok(rocket.manage(email))
        }))
        .manage(init_db_pool())
        .attach(cors)
        .attach(Template::fairing())
        .attach(https::ProductionHttpsRedirect {})
        .mount("/", routes![admin])
        .mount("/api", routes![
            login, logout, register, start_reset_password, reset_password,
            sign_user_upload, get_photos, get_photos_page,
            fetch_user_albums, create_album, fetch_album, add_photos_to_album, remove_photos_from_album,
            publish_album, get_published_albums, delete_published_album, toggle_published_album,
            get_published_photos
        ])
        .mount("/internal", routes![https::redirect_handler])
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
