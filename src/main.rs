use log::error;
use rocket::{
    fairing::{self, AdHoc},
    get, launch, routes, Build, Rocket,
};
use rocket_db_pools::{Connection, Database as R_Database};

mod auth;

use sqlx::Sqlite;

use crate::auth::users::User;

#[derive(R_Database)]
#[database("sqlite_db")]
pub struct Database(sqlx::SqlitePool);

pub type DbConnection = Connection<Database>;
pub type DbPool = sqlx::pool::PoolConnection<Sqlite>;

#[get("/")]
async fn index() -> &'static str {
    "Hello, world!"
}

// Testing the User request guard
#[get("/username")]
async fn username(user: &User) -> String {
    user.display_name
        .clone()
        .unwrap_or(user.default_display_name.clone())
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match Database::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to initialize SQLx database: {}", e);
                Err(rocket)
            }
        },
        None => Err(rocket),
    }
}

#[launch]
fn rocket() -> _ {
    if cfg!(debug_assertions) {
        dotenvy::from_filename(".dev.env").ok();
    }

    dotenvy::dotenv().ok();

    rocket::build()
        .mount("/", routes![index, username,])
        .attach(Database::init())
        .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        .attach(auth::stage())
}
