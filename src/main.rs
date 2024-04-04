use rocket::{catch, catchers, get, http::Status, launch, routes, Request};
use rocket_dyn_templates::Template;

#[macro_use]
extern crate rocket_dyn_templates;

mod auth;
mod contests;
mod csp;
mod db;
mod problems;
mod run;
mod serve;
mod settings;
#[macro_use]
mod template;
mod times;

use crate::auth::users::User;

#[get("/")]
async fn index(user: Option<&User>) -> Template {
    let ctx = context_with_base!(user,);
    Template::render("index", ctx)
}

#[get("/styles")]
async fn styles(user: Option<&User>) -> Template {
    let ctx = context_with_base!(user,);
    Template::render("styles", ctx)
}

#[catch(default)]
fn error(status: Status, _request: &Request) -> Template {
    let message = status.to_string();
    let code = status.code;
    Template::render(
        "error",
        context! { message, code, version: env!("CARGO_PKG_VERSION") },
    )
}

#[launch]
fn rocket() -> _ {
    if cfg!(debug_assertions) {
        dotenvy::from_filename(".dev.env").ok();
    }

    dotenvy::dotenv().ok();

    rocket::build()
        .mount("/", routes![index, styles])
        .register("/", catchers![error])
        .attach(csp::stage())
        .attach(db::stage())
        .attach(times::stage())
        .attach(template::stage())
        .attach(serve::stage())
        .attach(auth::stage())
        .attach(settings::stage())
        .attach(contests::stage())
        .attach(problems::stage())
}
