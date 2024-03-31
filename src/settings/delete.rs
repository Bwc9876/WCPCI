use log::error;
use rocket::{
    get,
    http::{CookieJar, Status},
    post,
    response::Redirect,
};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        sessions::Session,
        users::User,
    },
    context_with_base_authed,
    db::DbConnection,
};

#[get("/account/delete")]
pub async fn delete_user_get(user: &User, _token: &CsrfToken) -> Template {
    let ctx = context_with_base_authed!(user,);
    Template::render("settings/delete", ctx)
}

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum DeleteUserResponse {
    Redirect(Redirect),
    Error(Status),
}

#[post("/account/delete")]
pub async fn delete_user_post(
    mut db: DbConnection,
    user: &User,
    cookies: &CookieJar<'_>,
    _token: &VerifyCsrfToken,
) -> DeleteUserResponse {
    if let Err(why) = user.delete(&mut db).await {
        error!("Failed to delete user {}: {:?}", user.id, why);
        DeleteUserResponse::Error(Status::InternalServerError)
    } else {
        cookies.remove_private(Session::TOKEN_COOKIE_NAME);
        DeleteUserResponse::Redirect(Redirect::to("/"))
    }
}
