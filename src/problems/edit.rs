use log::error;
use rocket::{
    form::{Contextual, Form},
    get,
    http::Status,
    post,
};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::User,
    },
    context_with_base_authed,
    db::DbConnection,
    template::{FormStatus, FormTemplateObject},
};

use super::{cases::TestCase, Problem, ProblemForm, ProblemFormTemplate};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum ProblemEditResponse {
    Form(Template),
    NotFound(Status),
}

#[get("/<id>/edit")]
pub async fn edit_problem_get(
    user: &User,
    mut db: DbConnection,
    id: i64,
    _token: &CsrfToken,
) -> ProblemEditResponse {
    if let Some(problem) = Problem::get(&mut db, id).await {
        let test_cases = TestCase::get_for_problem(&mut db, problem.id)
            .await
            .unwrap_or_default();
        let form_template = ProblemFormTemplate {
            problem: Some(&problem),
            test_cases: test_cases.iter().map(TestCase::to_form).collect(),
        };
        let form = FormTemplateObject::get(form_template);
        ProblemEditResponse::Form(Template::render(
            "problems/edit",
            context_with_base_authed!(user, form, problem_name: problem.name, problem_id: problem.id),
        ))
    } else {
        ProblemEditResponse::NotFound(Status::NotFound)
    }
}

#[post("/<id>/edit", data = "<form>")]
pub async fn edit_problem_post(
    id: i64,
    user: &User,
    form: Form<Contextual<'_, ProblemForm<'_>>>,
    _token: &VerifyCsrfToken,
    mut db: DbConnection,
) -> ProblemEditResponse {
    if let Some(mut problem) = Problem::get(&mut db, id).await {
        let mut test_cases = TestCase::get_for_problem(&mut db, problem.id)
            .await
            .unwrap_or_default();
        let form_template = ProblemFormTemplate {
            problem: Some(&problem),
            test_cases: test_cases.iter().map(TestCase::to_form).collect(),
        };

        let original_name = problem.name.clone();

        if let Some(ref value) = form.value {
            problem.name = value.name.to_string();
            problem.description = value.description.to_string();
            problem.cpu_time = value.cpu_time;
            let res = sqlx::query!(
                "UPDATE problem SET name = ?, description = ?, cpu_time = ? WHERE id = ?",
                problem.name,
                problem.description,
                problem.cpu_time,
                problem.id
            )
            .execute(&mut **db)
            .await;
            let status = if let Err(why) = res {
                error!("Failed to update problem: {:?}", why);
                FormStatus::Error
            } else {
                test_cases = TestCase::from_vec(problem.id, &value.test_cases);
                let backup_cases = test_cases.clone();
                if let Ok(new_cases) = TestCase::save_for_problem(&mut db, test_cases).await {
                    test_cases = new_cases;
                    FormStatus::Success
                } else {
                    test_cases = backup_cases;
                    FormStatus::Error
                }
            };
            let form_template = ProblemFormTemplate {
                problem: Some(&problem),
                test_cases: test_cases.iter().map(TestCase::to_form).collect(),
            };
            let mut form_ctx = FormTemplateObject::get(form_template);
            form_ctx.status = status;
            ProblemEditResponse::Form(Template::render(
                "problems/edit",
                context_with_base_authed!(user, form: form_ctx, problem_name: original_name, problem_id: problem.id),
            ))
        } else {
            let form_ctx = FormTemplateObject::from_rocket_context(form_template, &form.context);
            ProblemEditResponse::Form(Template::render(
                "problems/edit",
                context_with_base_authed!(user, form: form_ctx, problem_name: original_name, problem_id: problem.id),
            ))
        }
    } else {
        ProblemEditResponse::NotFound(Status::NotFound)
    }
}
