pub mod album;
mod pagination;
pub mod photo;
pub mod publishing;
pub mod user;
pub mod schema;

pub use self::pagination::{Page, Pagination};
pub use diesel::PgConnection;

use rocket_contrib_codegen::database;

///! see: https://rocket.rs/guide/state/#databases
#[database("photos")]
pub struct DbConn(PgConnection);

#[cfg(test)]
pub fn test_db() -> PgConnection {
    use std::env;
    use dotenv;
    use diesel::Connection;

    dotenv::dotenv().ok();
    // TODO: make this use the same config param as the rocket magic
    let db = env::var("DATABASE_URL").expect("missing database url");
    PgConnection::establish(&db).expect("couldn't connect to db")
}
