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
use photothing_api::web::Page;
use photothing_api::albums::Album;
use photothing_api::photos::Photo;

use rocket::local::Client;
use rocket::http::{Cookie, Status};
use serde_json::Value;
use chrono::prelude::*;

use utils::web::{get, post, put, delete, assert_user_cookie};
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

    // make sure the authenticated endpoints 401 json and not html
    let mut res = post(&client, "/api/albums/1/publish", &json!({"foo": "bar"}), None);
    let body = serde_json::from_str(&res.body_string().expect("missing body on login"))
        .expect("401 body parsing failed");
    match body {
        Value::Object(_map) => (),
        _ => assert!(false, "didn't get object back for 401 response"),
    }

    // register a new user & log in
    let res = register(&creds);
    assert_eq!(res.status(), Status::Ok);
    let res = login(&creds);
    assert_eq!(res.status(), Status::Ok);
    let login_cookie = assert_user_cookie(&res, true).expect("login cookie missing");
    let login_cookie = Cookie::parse_encoded(login_cookie).expect("login cookie parsing failed");

    // curry the login cookie into some more helper methods
    let get_photos = || get(&client, "/api/photos", login_cookie.clone());
    let create_photo = |body: &Value| post(&client, "/api/upload", body, Some(login_cookie.clone()));
    let create_album = |name: &str| post(&client, &format!("/api/albums?name={}", name), &json!({}), Some(login_cookie.clone()));
    let add_photos = |id: i32, photos: Vec<i32>| put(&client, &format!("/api/albums/{}/photos", id), &json!(photos), Some(login_cookie.clone()));
    let remove_photos = |id: i32, photos: Vec<i32>| delete(&client, &format!("/api/albums/{}/photos", id), &json!(photos), Some(login_cookie.clone()));
    let get_album_photos = |id: i32, photo_id: &i32| get(&client, &format!("/api/albums/{}?key={}", id, photo_id), login_cookie.clone());

    // check that there are no photos for brand-new users
    {
        let mut res = get_photos();
        assert_eq!(res.status(), Status::Ok);
        let empty_photos: Page<Photo> = serde_json::from_slice(&res.body_bytes().expect("body")).expect("server JSON valid");
        assert!(empty_photos.items.is_empty());
        assert_eq!(empty_photos.remaining, 0);
        assert!(empty_photos.next_key.is_none());
        assert!(empty_photos.key.is_none());
    }

    // set the user's subscription so we can upload photos
    let db = db::test_db();
    let user = User::for_update(&db, &email).expect("db").expect("found user");
    user.edit_subscription(&db, Some(NaiveDate::from_ymd(2075, 1, 1))).expect("db");

    // create a bunch of photos
    for i in 0..40 {
        let photo = json!({ "filename": format!("pic-{}.jpg", i), "file_type": "jpg" });
        let res = create_photo(&photo);
        assert_eq!(res.status(), Status::Ok);
    }

    // verify that we can retrieve photos from the API after creating them
    let photo_ids: Vec<i32>;
    {
        let mut res = get_photos();
        assert_eq!(res.status(), Status::Ok);
        let photos: Page<Photo> = serde_json::from_slice(&res.body_bytes().expect("body")).expect("server JSON valid");
        assert_eq!(photos.items.len(), 30);
        assert_eq!(photos.remaining, 10);
        assert!(photos.next_key.is_some(), "remaining key returned");
        assert!(photos.key.is_none(), "key not none for first page of photos");
        photo_ids = photos.map(|p| p.id).items[0..3].to_vec();
    }

    let mut res = create_album("baby%27s%20first%20%40lbum");
    assert_eq!(res.status(), Status::Ok);
    let album: Album = serde_json::from_slice(&res.body_bytes().expect("body")).expect("valid json");
    assert!(album.photos.is_empty());
    assert_eq!(album.name, Some(String::from("baby's first @lbum")));
    for entry in album.photos.items.iter() {
        assert!(entry.caption.is_none(), "no caption");
        let filename = entry.photo.attributes.get("filename").expect("missing filename");
        assert!(filename.starts_with("pic-"), "weird filename");
    }

    let mut res = add_photos(album.id, photo_ids.clone());
    let populated_album: Album = serde_json::from_slice(&res.body_bytes().expect("body")).expect("valid json");
    assert_eq!(album.id, populated_album.id, "right album added to");
    assert_eq!(populated_album.photos.items.len(), 3, "three photos added");

    // check that pagination is respected for photo albums
    {
        let mut res = get_album_photos(album.id, photo_ids.iter().next().unwrap());
        assert_eq!(res.status(), Status::Ok);
        let paginated: Album = serde_json::from_slice(&res.body_bytes().expect("body")).expect("valid json");
        assert_eq!(paginated.photos.items.len(), 2, "we skipped the first item");
    }

    let mut res = remove_photos(album.id, vec![photo_ids.into_iter().next().unwrap()]);
    let two_pics: Album = serde_json::from_slice(&res.body_bytes().expect("body")).expect("valid json");
    assert_eq!(two_pics.photos.items.len(), 2, "two photos remaining");
}
