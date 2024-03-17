use log::error;
use rocket::{
    form::{Contextual, Form},
    get, post,
    response::Redirect,
};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::User,
    },
    context_with_base_authed,
    db::DbConnection,
    template::FormTemplateObject,
};

use super::{cases::TestCase, Problem, ProblemForm, ProblemFormTemplate};

#[get("/new", rank = 5)]
pub fn new_problem_get(user: &User, _token: &CsrfToken) -> Template {
    let form_template = ProblemFormTemplate {
        problem: None,
        test_cases: vec![],
    };
    let form = FormTemplateObject::get(form_template);
    Template::render("problems/new", context_with_base_authed!(user, form))
}

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum ProblemNewResponse {
    Redirect(Redirect),
    Error(Template),
}

#[post("/new", data = "<form>", rank = 5)]
pub async fn new_problem_post(
    user: &User,
    form: Form<Contextual<'_, ProblemForm<'_>>>,
    _token: &VerifyCsrfToken,
    mut db: DbConnection,
) -> ProblemNewResponse {
    if let Some(ref value) = form.value {
        let problem = Problem::temp(value);
        let res = problem.write_to_db(&mut db).await;
        match res {
            Ok(problem) => {
                let test_cases = TestCase::from_vec(problem.id, &value.test_cases);
                if let Err(why) = TestCase::save_for_problem(&mut db, test_cases).await {
                    error!("Error saving test cases: {:?}", why);
                }
                ProblemNewResponse::Redirect(Redirect::to(format!("/problems/{}", problem.id)))
            }
            Err(why) => {
                error!("Error saving problem: {:?}", why);
                ProblemNewResponse::Redirect(Redirect::to("/problems"))
            }
        }
    } else {
        let form_template = ProblemFormTemplate {
            problem: None,
            test_cases: vec![],
        };
        let form = FormTemplateObject::from_rocket_context(form_template, &form.context);

        ProblemNewResponse::Error(Template::render(
            "problems/new",
            context_with_base_authed!(user, form),
        ))
    }
}
