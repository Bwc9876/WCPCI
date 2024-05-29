use rocket::{get, launch, routes};
use rocket_dyn_templates::Template;

#[macro_use]
extern crate rocket_dyn_templates;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate rocket;

mod admin;
mod auth;
mod contests;
mod csp;
mod db;
mod error;
mod leaderboard;
mod messages;
mod problems;
mod profile;
mod run;
mod serve;
mod settings;
#[macro_use]
mod template;
mod times;

use crate::auth::users::User;
use crate::error::prelude::*;

#[get("/")]
async fn index(user: Option<&User>) -> Template {
    let ctx = context_with_base!(user,);
    Template::render("index", ctx)
}

#[get("/md-help")]
async fn md_help(user: Option<&User>) -> Template {
    let ctx = context_with_base!(user,);
    Template::render("md_help", ctx)
}

#[launch]
fn rocket() -> _ {
    if cfg!(debug_assertions) {
        println!("Loading .dev.env...");
        dotenvy::from_filename(".dev.env").ok();
    }

    println!("Loading .env...");
    if let Err(why) = dotenvy::dotenv() {
        eprintln!("Failed to load .env: {}", why);
    }

    println!("Start of WCPC v{}", env!("CARGO_PKG_VERSION"));

    rocket::build()
        .mount("/", routes![index, md_help])
        .attach(error::stage())
        .attach(csp::stage())
        .attach(db::stage())
        .attach(times::stage())
        .attach(template::stage())
        .attach(serve::stage())
        .attach(auth::stage())
        .attach(settings::stage())
        .attach(admin::stage())
        .attach(contests::stage())
        .attach(problems::stage())
        .attach(leaderboard::stage())
        .attach(profile::stage())
}
