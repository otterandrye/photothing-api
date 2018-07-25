extern crate photothing_api;
#[macro_use] extern crate serde_json;
extern crate reqwest;
extern crate rocket;

//use reqwest::{StatusCode, Client};
use rocket::local::{Client, LocalResponse};
use rocket::http::{Cookie, ContentType, Status};

fn post<'a, 'b, 'c>(client: &'a Client, endpoint: &'b str, body: &'c serde_json::Value) -> LocalResponse<'a> {
    client.post(endpoint.to_string())
        .header(ContentType::JSON)
        .body(format!("{}", body))
        .dispatch()
}

fn assert_user_cookie<'a, 'b>(res: &'a LocalResponse, expected: bool) -> Option<String> {
    println!("Got headers: {:?}", res.headers());
    for h in res.headers().iter() {
        if h.name() == "Set-Cookie" {
            let found = h.value().starts_with("u=") && !h.value().starts_with("u=; Path=/;");
            assert_eq!(found, expected, "login cookie in wrong state (actual, expected)");
            if found {
                return Some(h.value().to_string());
            } else {
                return None;
            }
        }
    }
    assert!(!expected);
    None
}

#[test]
#[ignore]
fn user_registration_login() {
    let client = Client::new(photothing_api::web::rocket()).expect("rocket launched");

    // http helper methods
    let login = |body: &serde_json::Value|    post(&client, "/api/login", body);
    let logout = ||                           post(&client, "/api/logout", &json!({}));
    let register = |body: &serde_json::Value| post(&client, "/api/register", body);

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

    // register a new user
    let res = register(&creds);
    assert_eq!(res.status(), Status::Ok);
    // registration != login, makes duplicate email handling possible
    assert_user_cookie(&res, false);

    let res = login(&invalid_creds);
    assert_eq!(res.status(), Status::Unauthorized, "bad creds");
    assert_user_cookie(&res, false);
    // and log in
    let res = login(&creds);
    assert_eq!(res.status(), Status::Ok);
    let login_cookie = assert_user_cookie(&res, true).expect("login cookie missing");
    let login_cookie = Cookie::parse_encoded(login_cookie).expect("login cookie parsing failed");

    // if registering w/ an in-use email we shouldn't expose an error
    let mut res = register(&creds);
    assert_eq!(res.status(), Status::Ok);
    assert_user_cookie(&res, false);
    assert_eq!(res.body_string().expect("missing body on error response"),
               r#"{"email":"nathan@chemist.com","error":null}"#);

    // finally, check that an authenticated request works
    let res =  client.get("/api/photos")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .dispatch();
    assert_eq!(res.status(), Status::Ok);
    // and that one requiring a subscription returns Forbidden
    let res =  client.post("/api/upload")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .body(format!("{}", json!({ "filename": "foo", "file_type": "jpg" })))
            .dispatch();
    assert_eq!(res.status(), Status::Forbidden);

    // and that logout succeeds & clears the user cookie
    let res =  client.post("/api/logout")
            .header(ContentType::JSON)
            .cookie(login_cookie.clone())
            .body("{}")
            .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_user_cookie(&res, false);
}
