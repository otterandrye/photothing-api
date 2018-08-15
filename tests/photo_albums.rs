// for every integration test
extern crate photothing_api;
extern crate diesel;
extern crate dotenv;
#[macro_use] extern crate serde_json;
extern crate rocket;

mod utils;

// just for this test
extern crate chrono;
use photothing_api::db::user::User;

use rocket::local::Client;
use rocket::http::{Cookie, Status};
use serde_json::Value;
use chrono::prelude::*;

use utils::web::{get, post, assert_user_cookie};
use utils::db;

#[test]
#[ignore]
fn photo_albums() {
    let client = Client::new(photothing_api::web::rocket()).expect("rocket launched");

    // http helper methods
    let login = |body: &Value|    post(&client, "/api/login", body, None);
    let register = |body: &Value| post(&client, "/api/register", body, None);

    let email = String::from("photo@testing.com");
    let creds = json!({ "email": &email, "password": "ninja truck bar fight" });

    let res = register(&creds);
    assert_eq!(res.status(), Status::Ok);
    let res = login(&creds);
    assert_eq!(res.status(), Status::Ok);
    let login_cookie = assert_user_cookie(&res, true).expect("login cookie missing");
    let login_cookie = Cookie::parse_encoded(login_cookie).expect("login cookie parsing failed");

    let get_photos = || get(&client, "/api/photos", login_cookie.clone());
    let create_photo = |body: &Value| post(&client, "/api/upload", body, Some(login_cookie.clone()));

    let res = get_photos();
    assert_eq!(res.status(), Status::Ok);
    // TODO: deserialize & inspect

    // set the user's subscription so we can upload photos
    let db = db::test_db();
    let user = User::for_update(&db, &email).expect("db").expect("found user");
    user.edit_subscription(&db, Some(NaiveDate::from_ymd(2075, 1, 1))).expect("db");

    for i in 0..5 {
        let photo = json!({ "filename": format!("pic-{}.jpg", i), "file_type": "jpg" });
        let res = create_photo(&photo);
        assert_eq!(res.status(), Status::Ok);
    }

    // TODO: pagination here rather than everything
    let res = get_photos();
    assert_eq!(res.status(), Status::Ok);
    // TODO: deserialize & inspect

    // TODO: create album
    // TODO: add two pictures to album
    // TODO: verify we can fetch pictures in album with pagination
    // TODO: remove one picture from album
    // TODO: fetch again to verify
}
