use log::error;
use rocket::{
    form::{Contextual, Form},
    get,
    http::Status,
    post,
    response::Redirect,
    State,
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
    run::ManagerHandle,
    template::FormTemplateObject,
};

use super::{cases::TestCase, Problem, ProblemForm, ProblemFormTemplate};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum ProblemEditResponse {
    Form(Template),
    Redirect(Redirect),
    NotFound(Status),
}

#[get("/<contest_id>/problems/<slug>/edit")]
pub async fn edit_problem_get(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    mut db: DbConnection,
    slug: &str,
    _token: &CsrfToken,
) -> ProblemEditResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return ProblemEditResponse::NotFound(Status::Forbidden);
    }
    if let Some(problem) = Problem::get(&mut db, contest_id, slug).await {
        let test_cases = TestCase::get_for_problem(&mut db, problem.id)
            .await
            .unwrap_or_default();
        let form_template = ProblemFormTemplate {
            problem: Some(&problem),
            test_cases: test_cases.iter().map(TestCase::to_form).collect(),
        };
        let contest_name = Contest::get(&mut db, contest_id)
            .await
            .map(|c| c.name)
            .unwrap_or_default();
        let form = FormTemplateObject::get(form_template);
        ProblemEditResponse::Form(Template::render(
            "problems/edit",
            context_with_base_authed!(user, form, contest_name, contest_id, problem),
        ))
    } else {
        ProblemEditResponse::NotFound(Status::NotFound)
    }
}

// Has to be a large number of parameters because this is Rocket
#[allow(clippy::too_many_arguments)]
#[post("/<contest_id>/problems/<slug>/edit", data = "<form>")]
pub async fn edit_problem_post(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    slug: &str,
    form: Form<Contextual<'_, ProblemForm<'_>>>,
    _token: &VerifyCsrfToken,
    manager: &State<ManagerHandle>,
    mut db: DbConnection,
) -> ProblemEditResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return ProblemEditResponse::NotFound(Status::Forbidden);
    }

    if let Some(mut problem) = Problem::get(&mut db, contest_id, slug).await {
        let test_cases = TestCase::get_for_problem(&mut db, problem.id)
            .await
            .unwrap_or_default();
        let form_template = ProblemFormTemplate {
            problem: Some(&problem),
            test_cases: test_cases.iter().map(TestCase::to_form).collect(),
        };

        let original_name = problem.name.clone();

        if let Some(ref value) = form.value {
            problem.name = value.name.to_string();
            problem.slug = slug::slugify(value.name);
            problem.description = value.description.to_string();
            problem.cpu_time = value.cpu_time;
            let res = problem.update(&mut db).await;
            if let Err(why) = res {
                error!("Failed to update problem: {:?}", why);
            } else {
                let test_cases = TestCase::from_vec(problem.id, &value.test_cases);
                if let Err(why) = TestCase::save_for_problem(&mut db, test_cases).await {
                    error!("Failed to update test cases: {:?}", why);
                } else {
                    let mut manager = manager.lock().await;
                    manager.update_problem(problem.id).await;
                }
            };
            ProblemEditResponse::Redirect(Redirect::to(format!(
                "/contests/{}/problems/{}",
                contest_id, problem.slug
            )))
        } else {
            let form_ctx = FormTemplateObject::from_rocket_context(form_template, &form.context);
            let contest_name = Contest::get(&mut db, contest_id)
                .await
                .map(|c| c.name)
                .unwrap_or_default();
            ProblemEditResponse::Form(Template::render(
                "problems/edit",
                context_with_base_authed!(user, form: form_ctx, contest_id, contest_name, problem, problem_name: original_name),
            ))
        }
    } else {
        ProblemEditResponse::NotFound(Status::NotFound)
    }
}
