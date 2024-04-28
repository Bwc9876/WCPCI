use rocket::{
    form::{Contextual, Error, Form},
    get,
    http::Status,
    post, State,
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
    error::prelude::*,
    messages::Message,
    run::ManagerHandle,
    template::FormTemplateObject,
};

use super::{cases::TestCase, Problem, ProblemForm, ProblemFormTemplate};

#[get("/<contest_id>/problems/<slug>/edit")]
pub async fn edit_problem_get(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    mut db: DbConnection,
    slug: &str,
    _token: &CsrfToken,
) -> ResultResponse<Template> {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await?
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return Err(Status::Forbidden.into());
    }
    let problem = Problem::get_or_404(&mut db, contest_id, slug).await?;
    let test_cases = TestCase::get_for_problem(&mut db, problem.id).await?;
    let form_template = ProblemFormTemplate {
        problem: Some(&problem),
        test_cases: test_cases.iter().map(TestCase::to_form).collect(),
    };
    let contest = Contest::get_or_404(&mut db, contest_id).await?;
    let form = FormTemplateObject::get(form_template);
    Ok(Template::render(
        "problems/edit",
        context_with_base_authed!(user, form, contest, problem),
    ))
}

// Has to be a large number of parameters because this is Rocket
#[allow(clippy::too_many_arguments)]
#[post("/<contest_id>/problems/<slug>/edit", data = "<form>")]
pub async fn edit_problem_post(
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
    slug: &str,
    mut form: Form<Contextual<'_, ProblemForm<'_>>>,
    _token: &VerifyCsrfToken,
    manager: &State<ManagerHandle>,
    mut db: DbConnection,
) -> FormResponse {
    let is_judge = Participant::get(&mut db, contest_id, user.id)
        .await?
        .map(|p| p.is_judge)
        .unwrap_or(false);
    let is_admin = admin.is_some();
    if !is_judge && !is_admin {
        return Err(Status::Forbidden.into());
    }

    let mut problem = Problem::get_or_404(&mut db, contest_id, slug).await?;
    let test_cases = TestCase::get_for_problem(&mut db, problem.id)
        .await
        .unwrap_or_default();
    let form_template = ProblemFormTemplate {
        problem: Some(&problem),
        test_cases: test_cases.iter().map(TestCase::to_form).collect(),
    };

    let original_name = problem.name.clone();
    if let Some(ref value) = form.value {
        let new_slug = slug::slugify(value.name);

        if Problem::slug_exists(&mut db, &new_slug, contest_id, Some(problem.id)).await? {
            let err = Error::validation("Problem with this name already exists").with_name("name");
            form.context.push_error(err);
        } else if value.test_cases.is_empty() {
            let err =
                Error::validation("At least one test case is required").with_name("test_cases");
            form.context.push_error(err);
        } else {
            problem.name = value.name.to_string();
            problem.slug = new_slug;
            problem.description = value.description.to_string();
            problem.cpu_time = value.cpu_time;
            problem.update(&mut db).await?;
            let test_cases = TestCase::from_vec(problem.id, &value.test_cases);
            TestCase::save_for_problem(&mut db, problem.id, test_cases).await?;
            let mut manager = manager.lock().await;
            manager.update_problem(problem.id).await;
            return Ok(Message::success("Problem Updated").to(&format!(
                "/contests/{}/problems/{}",
                contest_id, problem.slug
            )));
        }
    }

    let form_ctx = FormTemplateObject::from_rocket_context(form_template, &form.context);
    let contest = Contest::get(&mut db, contest_id).await.unwrap();
    Err(Template::render(
        "problems/edit",
        context_with_base_authed!(user, form: form_ctx, contest, problem, problem_name: original_name),
    ).into())
}
