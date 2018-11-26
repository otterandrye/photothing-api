use rocket::{Rocket, Request, Data};
use rocket::config::Environment;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fairing::AdHoc;
use rocket::response::Redirect;
use rocket::codegen::uri;

/// This fairing redirects HTTP requests to HTTPS on heroku
pub struct ProductionHttpsRedirect;

impl Fairing for ProductionHttpsRedirect {
    fn info(&self) -> Info {
        Info {
            name: "ProductionHttpsRedirector",
            kind: Kind::Attach
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        match rocket.config().environment {
            Environment::Production => {
                // hack! use a config param since rocket requests don't contains the hostname
                let host = rocket.config().get_str("api_host")
                    .expect("missing api host").to_owned();
                let rocket = rocket.attach(AdHoc::on_request("http->https", https_redirector(host)));
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
            req.set_uri(uri!(redirect_handler: host.clone(), uri));
        }
    }
}

/// This must be attached to rocket under '/internal' to handle the redirect routing
#[get("/interna/https-redirect?<host>&<to>")]
pub fn redirect_handler(host: String, to: String) -> Redirect {
    Redirect::permanent(format!("https://{}{}", &host, &to))
}
