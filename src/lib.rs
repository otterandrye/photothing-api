#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate bcrypt;
extern crate chrono;
#[macro_use] extern crate diesel;
extern crate dotenv;
extern crate futures;
#[macro_use] extern crate log;
extern crate mailchecker;
extern crate r2d2;
extern crate rocket;
extern crate rocket_contrib;
extern crate rocket_cors;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_s3;

#[cfg(test)]
#[macro_use] extern crate serde_json;

#[cfg(test)]
extern crate reqwest;
#[cfg(test)]
extern crate rand;

mod auth;
mod db;
mod s3;
mod util;
pub mod web;
