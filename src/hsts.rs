/// HTTP Strict-Transport-Security header as a rocket fairing
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security
/// https://api.rocket.rs/v0.4/rocket/fairing/trait.Fairing.html

use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::hyper::header::StrictTransportSecurity;

const MONTH_SECS: u64 = 60*60*24*31;

pub struct StsOptions {
    pub expire_time: u64,
    pub include_subdomains: bool,
}

impl Default for StsOptions {
    fn default() -> Self {
        StsOptions {
            expire_time: MONTH_SECS,
            include_subdomains: false,
        }
    }
}

struct StsFairing {
    options: StsOptions
}

impl Fairing for StsFairing {
    fn info(&self) -> Info {
        Info {
            name: "HSTS",
            kind: Kind::Response
        }
    }

    fn on_response(&self, _req: &Request, res: &mut Response) {
        let sts = if self.options.include_subdomains {
            StrictTransportSecurity::including_subdomains(self.options.expire_time)
        } else {
            StrictTransportSecurity::excluding_subdomains(self.options.expire_time)
        };
        res.set_header(sts);
    }
}

/// Add the STS header to all outgoing responses
pub fn sts_header() -> impl Fairing {
    StsFairing { options: StsOptions::default() }
}

/// Add the STS header to all outgoing responses
/// Customize your expire time & preload/subdomain options
#[allow(dead_code)]
pub fn sts_header_opts(options: StsOptions) -> impl Fairing {
    StsFairing { options }
}
