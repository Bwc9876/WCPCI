use log::error;
use rocket::{
    form::{Contextual, Error, Form},
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
    Error(Status),
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
    if Contest::get(&mut db, contest_id).await.is_none() {
        return ProblemNewGetResponse::Error(Status::NotFound);
    }
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
        let contest = Contest::get(&mut db, contest_id).await.unwrap();
        let form = FormTemplateObject::get(form_template);
        ProblemNewGetResponse::Template(Template::render(
            "problems/new",
            context_with_base_authed!(user, contest, form),
        ))
    } else {
        ProblemNewGetResponse::Error(Status::Forbidden)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum ProblemNewPostResponse {
    Redirect(Redirect),
    Template(Template),
    Error(Status),
}

#[post("/<contest_id>/problems/new", data = "<form>", rank = 5)]
pub async fn new_problem_post(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    mut form: Form<Contextual<'_, ProblemForm<'_>>>,
    _token: &VerifyCsrfToken,
    mut db: DbConnection,
) -> ProblemNewPostResponse {
    if Contest::get(&mut db, contest_id).await.is_none() {
        return ProblemNewPostResponse::Error(Status::NotFound);
    }
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return ProblemNewPostResponse::Error(Status::Forbidden);
    }
    if let Some(ref value) = form.value {
        let problem = Problem::temp(contest_id, value);
        if Problem::slug_exists(&mut db, &problem.slug, contest_id, None).await {
            let err = Error::validation("Problem with this name already exists").with_name("name");
            form.context.push_error(err);
        } else if value.test_cases.is_empty() {
            let err =
                Error::validation("At least one test case is required").with_name("test_cases");
            form.context.push_error(err);
        } else {
            let res = problem.insert(&mut db).await;
            return match res {
                Ok(problem) => {
                    let test_cases = TestCase::from_vec(problem.id, &value.test_cases);
                    if let Err(why) =
                        TestCase::save_for_problem(&mut db, problem.id, test_cases).await
                    {
                        error!("Error saving test cases: {:?}", why);
                        ProblemNewPostResponse::Error(Status::InternalServerError)
                    } else {
                        ProblemNewPostResponse::Redirect(Redirect::to(format!(
                            "/contests/{contest_id}/problems/{}",
                            problem.slug
                        )))
                    }
                }
                Err(why) => {
                    error!("Error saving problem: {:?}", why);
                    ProblemNewPostResponse::Error(Status::InternalServerError)
                }
            };
        }
    }

    let form_template = ProblemFormTemplate {
        problem: None,
        test_cases: vec![],
    };
    let form = FormTemplateObject::from_rocket_context(form_template, &form.context);

    let contest = Contest::get(&mut db, contest_id).await.unwrap();

    ProblemNewPostResponse::Template(Template::render(
        "problems/new",
        context_with_base_authed!(user, contest, form),
    ))
}
