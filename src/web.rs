use dotenv;
use harsh::Harsh;
use rocket::{ignite, Rocket, State, http::Cookies, request::Form};
use rocket_codegen::routes;
use rocket_contrib::json::Json;
use rocket_contrib::templates::Template;
use serde_json::{Value, json};

use crate::db::{DbConn, Pagination};
use crate::email::Emailer;
use crate::errors::ApiError;
use crate::s3::{S3Access, UploadRequest};
use crate::auth;
use crate::auth::guards::{User, Subscriber, Admin};
use crate::albums;
use crate::config;
use crate::hsts;
use crate::https;
use crate::photos;

pub use crate::db::Page;

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
    let photos = photos::user_photos(&user, &db, s3.inner(), Pagination::first())?;
    Ok(Json(photos))
}

#[get("/photos?<page..>")]
fn get_photos_page(user: User, s3: State<S3Access>, db: DbConn, page: Form<Pagination>) -> Api<Page<photos::Photo>> {
    let photos = photos::user_photos(&user, &db, s3.inner(), page.into_inner())?;
    Ok(Json(photos))
}

#[get("/albums?<page..>")]
fn fetch_user_albums(user: User, db: DbConn, page: Form<Pagination>) -> Api<Page<albums::Album>> {
    let user_albums = albums::user_albums(&db, &user, page.into_inner())?;
    Ok(Json(user_albums))
}

#[post("/albums?<details..>")]
fn create_album(user: User, db: DbConn, details: Form<albums::NewAlbum>) -> Api<albums::Album> {
    let album = albums::create_album(&db, &user, details.into_inner())?;
    Ok(Json(album))
}

#[get("/albums/<id>?<page..>")]
fn fetch_album(user: User, s3: State<S3Access>, db: DbConn, id: i32, page: Form<Pagination>) -> Api<albums::Album> {
    let album = albums::fetch_album(&db, &user, s3.inner(), id, page.into_inner())?;
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

#[get("/published/<hash_id>?<page..>")]
fn get_published_photos(db: DbConn, s3: State<S3Access>, harsh: State<Harsh>, hash_id: String, page: Form<Pagination>) -> Api<albums::Album> {
    // Note: not an authenticated endpoint!
    let photos = albums::get_published_photos(&db, s3.inner(), harsh.inner(), hash_id, page.into_inner())?;
    Ok(Json(photos))
}

#[delete("/published/<hash_id>")]
fn delete_published_album(user: User, db: DbConn, harsh: State<Harsh>, hash_id: String) -> Api<bool> {
    albums::delete_published_album(&db, &user, harsh.inner(), hash_id)?;
    Ok(Json(true))
}

#[post("/published/<hash_id>?<active>")]
fn toggle_published_album(db: DbConn, user: User, harsh: State<Harsh>, hash_id: String, active: bool) -> Api<bool> {
    albums::toggle_published_album(&db, &user, harsh.inner(), hash_id, active)?;
    Ok(Json(true))
}

#[get("/admin")]
fn admin(_admin: Admin, s3: State<S3Access>, db: DbConn) -> Result<Template, ApiError> {
    let context = create::admin::fetch_dashboard(&s3.inner(), &db)?;
    Ok(Template::render("admin", &context))
}

#[get("/me")]
fn me(user: User) -> Api<auth::UserResponse> {
    Ok(Json(auth::UserResponse::new(user)))
}

#[catch(401)]
fn unauthorized() -> ApiError {
    ApiError::unauthorized()
}

#[catch(404)]
fn not_found() -> Api<()> {
    ApiError::not_found(None, "Endpoint not found".to_string())
}

// Main entry that creates the web application, connects to the database and binds the web routes
pub fn rocket() -> Rocket {
    dotenv::dotenv().ok(); // read from a .env file if one is present
    ignite()
        .attach(config::s3())
        .attach(config::harsher())
        .attach(config::email())
        .attach(config::cors())
        .attach(DbConn::fairing())
        .attach(Template::fairing())
        .attach(https::ProductionHttpsRedirect {})
        .attach(hsts::sts_header())
        .register(catchers![unauthorized, not_found])
        .mount("/", routes![admin, https::redirect_handler])
        .mount("/api", routes![
            login, logout, register, start_reset_password, reset_password, me,
            sign_user_upload, get_photos, get_photos_page,
            fetch_user_albums, create_album, fetch_album, add_photos_to_album, remove_photos_from_album,
            publish_album, get_published_albums, delete_published_album, toggle_published_album,
            get_published_photos
        ])
}
