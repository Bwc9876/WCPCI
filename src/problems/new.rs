use log::error;
use rocket::{
    form::{Contextual, Form},
    get,
    http::Status,
    post,
    response::Redirect,
};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::{Admin, User},
    },
    contests::{Contest, Participant},
    context_with_base_authed,
    db::DbConnection,
    template::FormTemplateObject,
};

use super::{cases::TestCase, Problem, ProblemForm, ProblemFormTemplate};

#[derive(Responder)]
pub enum ProblemNewGetResponse {
    NotFound(Status),
    Template(Template),
}

#[get("/<contest_id>/problems/new", rank = 1)]
pub async fn new_problem_get(
    mut db: DbConnection,
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    _token: &CsrfToken,
) -> ProblemNewGetResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if is_judge || is_admin {
        let form_template = ProblemFormTemplate {
            problem: None,
            test_cases: vec![],
        };
        let contest_name = Contest::get(&mut db, contest_id)
            .await
            .map(|c| c.name)
            .unwrap_or_default();
        let form = FormTemplateObject::get(form_template);
        ProblemNewGetResponse::Template(Template::render(
            "problems/new",
            context_with_base_authed!(user, contest_name, contest_id, form),
        ))
    } else {
        ProblemNewGetResponse::NotFound(Status::Forbidden)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum ProblemNewPostResponse {
    Redirect(Redirect),
    Error(Template),
    NotFound(Status),
}

#[post("/<contest_id>/problems/new", data = "<form>", rank = 5)]
pub async fn new_problem_post(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    form: Form<Contextual<'_, ProblemForm<'_>>>,
    _token: &VerifyCsrfToken,
    mut db: DbConnection,
) -> ProblemNewPostResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return ProblemNewPostResponse::NotFound(Status::Forbidden);
    }
    if let Some(ref value) = form.value {
        let problem = Problem::temp(contest_id, value);
        let res = problem.insert(&mut db).await;
        match res {
            Ok(problem) => {
                let test_cases = TestCase::from_vec(problem.id, &value.test_cases);
                if let Err(why) = TestCase::save_for_problem(&mut db, test_cases).await {
                    error!("Error saving test cases: {:?}", why);
                }
                ProblemNewPostResponse::Redirect(Redirect::to(format!(
                    "/contests/{contest_id}/problems/{}",
                    problem.slug
                )))
            }
            Err(why) => {
                error!("Error saving problem: {:?}", why);
                ProblemNewPostResponse::Redirect(Redirect::to(format!(
                    "/contest/{contest_id}/problems"
                )))
            }
        }
    } else {
        let form_template = ProblemFormTemplate {
            problem: None,
            test_cases: vec![],
        };
        let form = FormTemplateObject::from_rocket_context(form_template, &form.context);

        let contest_name = Contest::get(&mut db, contest_id)
            .await
            .map(|c| c.name)
            .unwrap_or_default();

        ProblemNewPostResponse::Error(Template::render(
            "problems/new",
            context_with_base_authed!(user, contest_id, contest_name, form),
        ))
    }
}
