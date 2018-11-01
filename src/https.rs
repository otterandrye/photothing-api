use rocket::{Rocket, Request, Data};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fairing::AdHoc;
use rocket::config::Environment;
use rocket::response::Redirect;

pub struct ProductionHttpsRedirect;

impl Fairing for ProductionHttpsRedirect {
    fn info(&self) -> Info {
        Info {
            name: "ProductionOnly",
            kind: Kind::Attach
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        match rocket.config().environment {
            Environment::Production => {
                let host = rocket.config().get_str("api_host")
                    .expect("missing api host").to_owned();
                let rocket = rocket.attach(AdHoc::on_request(https_redirector(host)));
                Ok(rocket)
            },
            _ => Ok(rocket),
        }
    }
}

/// Returns a function that mutates the request uri if it wasn't https
fn https_redirector(host: String) -> impl Fn(&mut Request, &Data) {
    move |req, _data| {
        let is_https = match req.headers().get_one("x-forwarded-proto") {
            Some(scheme) if scheme == "https" => true,
            _ => false,
        };
        if !is_https {
            let uri = format!("{}", req.uri());
            req.set_uri(format!("/internal/https-redirect?host={}&to={}", host, uri));
        }
    }
}

#[derive(FromForm)]
pub struct RedirectParams {
    host: String,
    to: String,
}

/// This must be attached to rocket under '/internal' to handle the redirect routing
#[get("/https-redirect?<dest>")]
pub fn redirect_handler(dest: RedirectParams) -> Redirect {
    Redirect::permanent(&format!("https://{}{}", &dest.host, &dest.to))
}
