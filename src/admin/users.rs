use log::error;
use rocket::{get, http::Status, post, response::Redirect};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::{Admin, User},
    },
    context_with_base_authed,
    db::DbConnection,
};

#[get("/users")]
pub async fn users(mut db: DbConnection, user: &User, _admin: &Admin) -> Result<Template, Status> {
    let users = User::list(&mut db).await.map_err(|e| {
        error!("Failed to list users: {:?}", e);
        Status::InternalServerError
    })?;
    let ctx = context_with_base_authed!(user, users);
    Ok(Template::render("admin/users", ctx))
}

#[get("/users/<id>/delete")]
pub async fn delete_user_get(
    id: i64,
    mut db: DbConnection,
    user: &User,
    _admin: &Admin,
    _token: &CsrfToken,
) -> Result<Template, Status> {
    let target_user = User::get(&mut db, id).await.ok_or(Status::NotFound)?;
    let ctx = context_with_base_authed!(user, target_user);
    Ok(Template::render("admin/delete_user", ctx))
}

#[post("/users/<id>/delete")]
pub async fn delete_user_post(
    id: i64,
    mut db: DbConnection,
    _admin: &Admin,
    _token: &VerifyCsrfToken,
) -> Result<Redirect, Status> {
    let target_user = User::get(&mut db, id).await.ok_or(Status::NotFound)?;
    target_user.delete(&mut db).await.map_err(|e| {
        error!("Failed to delete user: {:?}", e);
        Status::InternalServerError
    })?;
    Ok(Redirect::to("/admin/users"))
}
