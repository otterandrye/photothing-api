// for every integration test
extern crate photothing_api;
extern crate diesel;
extern crate dotenv;
#[macro_use] extern crate serde_json;
extern crate rocket;

mod utils;

use rocket::local::Client;
use rocket::http::{Cookie, ContentType, Header, Status};
use serde_json::Value;

use utils::web::{get, post, assert_user_cookie};

#[test]
#[ignore]
fn user_registration_login() {
    let client = Client::new(photothing_api::web::rocket()).expect("rocket launched");

    // http helper methods
    let login = |body: &serde_json::Value|    post(&client, "/api/login", body, None);
    let logout = ||                           post(&client, "/api/logout", &json!({}), None);
    let register = |body: &serde_json::Value| post(&client, "/api/register", body, None);

    let email = "nathan@chemist.com";
    let password = "billy goat frialator";
    let creds = json!({ "email": email, "password": password });
    let invalid_creds = json!({ "email": email, "password": "not the password" });

    // can't login or out if the user isn't registered
    let res = login(&creds);
    // if this fails you may need to 'truncate table users' in your db
    // TODO: run tests in a separate/clean database
    assert_eq!(res.status(), Status::Unauthorized, "login: no user registered yet");
    let res = logout();
    assert_eq!(res.status(), Status::Unauthorized, "logout: no user registered");

    let invalid_email = json!({ "email": "foozip", "password": "someTiNg12##!" });
    let res = register(&invalid_email);
    assert_eq!(res.status(), Status::BadRequest, "register w/ invalid email");

    // register a new user
    let res = register(&creds);
    assert_eq!(res.status(), Status::Ok);
    // registration != login, makes duplicate email handling possible
    assert_user_cookie(&res, false);

    let res = login(&invalid_creds);
    assert_eq!(res.status(), Status::Unauthorized, "bad creds");
    assert_user_cookie(&res, false);

    // and log in
    let mut res = login(&creds);
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.content_type(), Some(ContentType::JSON));
    let login_cookie = assert_user_cookie(&res, true).expect("login cookie missing");
    let login_cookie = Cookie::parse_encoded(login_cookie).expect("login cookie parsing failed");
    let body = serde_json::from_str(&res.body_string().expect("missing body on login"))
        .expect("body parsing failed");
    let mut auth_header = None; // keep track of the header the server told us to use to login
    match body {
        Value::Object(map) => {
            match map.get("email") {
                Some(Value::String(email)) => assert_eq!(email, "nathan@chemist.com"),
                _ => assert!(false, "wrong email in login response"),
            };
            match map.get("pt_auth") {
                Some(Value::String(auth_value)) => assert_eq!(auth_value, login_cookie.value()),
                _ => assert!(false, "wrong auth value in login response"),
            };
            match map.get("header") {
                Some(Value::String(header)) => auth_header = Some(header.clone()),
                _ => assert!(false, "missing auth header in login response"),
            }
        },
        _ => assert!(false, "got badly formed login response body"),
    }
    let auth_header = auth_header.expect("didn't get auth header in login response");

    let res = get(&client, "/api/me", login_cookie.clone());
    assert_eq!(res.status(), Status::Ok, "/me api works when logged in");

    // if registering w/ an in-use email we shouldn't expose an error
    let mut res = register(&creds);
    assert_eq!(res.status(), Status::Ok);
    assert_user_cookie(&res, false);
    assert_eq!(res.body_string().expect("missing body on error response"),
               r#"{"email":"nathan@chemist.com"}"#);

    // finally, check that an authenticated request works
    // via cookie
    let res =  client.get("/api/photos?key=3&per_page=11")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .dispatch();
    assert_eq!(res.status(), Status::Ok, "cookie auth succeeded");
    // via header
    let login_header = Header::new(auth_header, login_cookie.value().to_string());
    let res =  client.get("/api/photos")
            .header(login_header)
            .dispatch();
    assert_eq!(res.status(), Status::Ok, "header auth succeeded");

    // and that one requiring a subscription returns Forbidden
    let res =  client.post("/api/upload")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .body(format!("{}", json!({ "filename": "foo", "file_type": "jpg" })))
            .dispatch();
    assert_eq!(res.status(), Status::Forbidden);

    // don't let normal users into the admin page
    let res =  client.get("/admin")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .dispatch();
    assert_eq!(res.status(), Status::NotFound);

    // and that logout succeeds & clears the user cookie
    let res =  client.post("/api/logout")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .body("{}")
            .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_user_cookie(&res, false);
}
