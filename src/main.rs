use rocket::{get, launch, routes};
use rocket_dyn_templates::context;
use rocket_dyn_templates::Template;

mod auth;
mod db;
mod serve;
mod template;

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
        .attach(template::stage())
        .attach(serve::stage())
        .attach(auth::stage())
}
