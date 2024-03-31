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

use super::Contest;

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum DeleteContestResponse {
    Template(Template),
    Error(Status),
    Redirect(Redirect),
}

#[get("/<contest_id>/delete")]
pub async fn delete_contest_get(
    contest_id: i64,
    mut db: DbConnection,
    _token: &CsrfToken,
    user: &User,
    _admin: &Admin,
) -> DeleteContestResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let ctx = context_with_base_authed!(user, contest);
        DeleteContestResponse::Template(Template::render("contests/delete", ctx))
    } else {
        DeleteContestResponse::Error(Status::NotFound)
    }
}

#[post("/<contest_id>/delete")]
pub async fn delete_contest_post(
    contest_id: i64,
    mut db: DbConnection,
    _token: &VerifyCsrfToken,
    _admin: &Admin,
) -> DeleteContestResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        if let Err(why) = contest.delete(&mut db).await {
            error!("Error deleting contest: {:?}", why);
            DeleteContestResponse::Error(Status::InternalServerError)
        } else {
            DeleteContestResponse::Redirect(Redirect::to("/contests"))
        }
    } else {
        DeleteContestResponse::Error(Status::NotFound)
    }
}
