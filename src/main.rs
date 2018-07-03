#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, rocket!"
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index])
}

fn main() {
    rocket().launch();
}


#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::local::Client;
    use rocket::http::Status;

    #[test]
    fn hello_world() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some("Hello, rocket!".into()));
    }
}
