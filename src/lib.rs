#![feature(plugin, custom_derive, proc_macro_hygiene, decl_macro)]
#![allow(proc_macro_derive_resolution_fallback)] // diesel warning, https://github.com/rust-lang/rust/issues/50504

extern crate bcrypt;
extern crate chrono;
#[macro_use] extern crate diesel;
extern crate dotenv;
extern crate futures;
extern crate harsh;
#[macro_use] extern crate itertools;
#[macro_use] extern crate log;
extern crate mailgun_v3;
extern crate mailchecker;
extern crate r2d2;
extern crate reqwest;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
extern crate rocket_cors;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_s3;
#[macro_use] extern crate serde_json;
extern crate uuid;
extern crate url;
extern crate zxcvbn;

#[cfg(test)]
extern crate rand;

mod admin;
pub mod albums;
mod auth;
mod config;
pub mod db;
mod email;
mod errors;
mod hsts;
mod https;
pub mod photos;
mod s3;
mod util;
pub mod web;
