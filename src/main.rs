use rocket::{get, launch, routes};
use rocket_dyn_templates::Template;

#[macro_use]
extern crate rocket_dyn_templates;

mod auth;
mod contests;
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

#[launch]
fn rocket() -> _ {
    if cfg!(debug_assertions) {
        dotenvy::from_filename(".dev.env").ok();
    }

    dotenvy::dotenv().ok();

    rocket::build()
        .mount("/", routes![index])
        .attach(db::stage())
        .attach(times::stage())
        .attach(template::stage())
        .attach(serve::stage())
        .attach(auth::stage())
        .attach(settings::stage())
        .attach(contests::stage())
        .attach(problems::stage())
}
