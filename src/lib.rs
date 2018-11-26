#![feature(plugin, custom_derive, custom_attribute, proc_macro_hygiene, decl_macro, nll)]
#![allow(proc_macro_derive_resolution_fallback)] // diesel warning, https://github.com/rust-lang/rust/issues/50504

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
