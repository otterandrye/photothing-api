// Helper utilities for integration testing

#[allow(dead_code)]
pub mod web {
    use rocket::local::{Client, LocalResponse};
    use rocket::http::{Cookie, ContentType};
    use serde_json::Value;

    pub fn post<'a, 'b, 'c>(client: &'a Client, endpoint: &'b str, body: &'c Value, login: Option<Cookie<'static>>) -> LocalResponse<'a> {
        let mut req = client.post(endpoint.to_string())
            .header(ContentType::JSON)
            .body(format!("{}", body));
        if login.is_some() {
            req = req.cookie(login.unwrap());
        }
        req.dispatch()
    }

    pub fn delete<'a, 'b, 'c>(client: &'a Client, endpoint: &'b str, body: &'c Value, login: Option<Cookie<'static>>) -> LocalResponse<'a> {
        let mut req = client.delete(endpoint.to_string())
            .header(ContentType::JSON)
            .body(format!("{}", body));
        if login.is_some() {
            req = req.cookie(login.unwrap());
        }
        req.dispatch()
    }

    pub fn put<'a, 'b, 'c>(client: &'a Client, endpoint: &'b str, body: &'c Value, login: Option<Cookie<'static>>) -> LocalResponse<'a> {
        let mut req = client.put(endpoint.to_string())
            .header(ContentType::JSON)
            .body(format!("{}", body));
        if login.is_some() {
            req = req.cookie(login.unwrap());
        }
        req.dispatch()
    }

    pub fn get<'a, 'b>(client: &'a Client, endpoint: &'b str, login: Cookie<'static>) -> LocalResponse<'a> {
        client.get(endpoint.to_string())
            .cookie(login)
            .header(ContentType::JSON)
            .dispatch()
    }

    pub fn assert_user_cookie<'a, 'b>(res: &'a LocalResponse, expected: bool) -> Option<String> {
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
}

#[allow(dead_code)]
pub mod db {
    use std::env;
    use dotenv;
    use diesel::{Connection, PgConnection};

    pub fn test_db() -> PgConnection {
        dotenv::dotenv().ok();
        let db = env::var("DATABASE_URL").expect("missing database url");
        PgConnection::establish(&db)
            .expect(&format!("Error connecting to {}", db))
    }
}
