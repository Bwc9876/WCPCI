use log::error;
use rocket::{get, http::Status, post, response::Redirect};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::{Admin, User},
    },
    contests::{Contest, Participant},
    context_with_base_authed,
    db::DbConnection,
};

use super::Problem;

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum ProblemDeleteResponse {
    Form(Template),
    Redirect(Redirect),
    Error(Status),
}

#[get("/<contest_id>/problems/<slug>/delete")]
pub async fn delete_problem_get(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    mut db: DbConnection,
    slug: &str,
    _token: &CsrfToken,
) -> ProblemDeleteResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return ProblemDeleteResponse::Error(Status::Forbidden);
    }
    if let Some(problem) = Problem::get(&mut db, contest_id, slug).await {
        let contest_name = Contest::get(&mut db, contest_id)
            .await
            .map(|c| c.name)
            .unwrap_or_default();
        ProblemDeleteResponse::Form(Template::render(
            "problems/delete",
            context_with_base_authed!(user, contest_name, contest_id, problem),
        ))
    } else {
        ProblemDeleteResponse::Error(Status::NotFound)
    }
}

#[post("/<contest_id>/problems/<slug>/delete")]
pub async fn delete_problem_post(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    slug: &str,
    _token: &VerifyCsrfToken,
    mut db: DbConnection,
) -> ProblemDeleteResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return ProblemDeleteResponse::Error(Status::Forbidden);
    }

    if let Some(problem) = Problem::get(&mut db, contest_id, slug).await {
        let res = problem.delete(&mut db).await;
        if let Err(e) = res {
            error!("Failed to delete problem: {}", e);
            ProblemDeleteResponse::Error(Status::InternalServerError)
        } else {
            ProblemDeleteResponse::Redirect(Redirect::to(format!(
                "/contests/{}/problems",
                contest_id
            )))
        }
    } else {
        ProblemDeleteResponse::Error(Status::NotFound)
    }
}
