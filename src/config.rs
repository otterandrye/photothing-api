use harsh::HarshBuilder;
use rocket::{http::Method, fairing::{AdHoc, Fairing}};
use rocket_cors::{Cors, AllowedOrigins, AllowedHeaders};

use crate::email::{init_emailer, dummy_emailer};
use crate::s3::S3Access;

///! stuff that needs configuration from the environment to work properly

pub fn s3() -> impl Fairing {
    AdHoc::on_attach("s3 config", |rocket| {
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
    })
}

pub fn harsher() -> impl Fairing {
    AdHoc::on_attach("harsher config", |rocket| {
        let salt = rocket.config().get_str("id_salt").expect("missing salt").to_owned();
        let harsh = HarshBuilder::new().salt(salt).length(4)
            .init().expect("couldn't init id hasher");
        Ok(rocket.manage(harsh))
    })
}

pub fn email() -> impl Fairing {
    AdHoc::on_attach("email config", |rocket| {
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
    })
}

pub fn cors() -> Cors {
    Cors {
        allowed_origins: AllowedOrigins::all(), // TODO: put into configuration
        allowed_methods: vec![Method::Get, Method::Post, Method::Put].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
}
