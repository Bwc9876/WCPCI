use rocket::{get, http::Status, State};
use rocket_dyn_templates::Template;

use crate::{auth::users::User, context_with_base, db::DbConnection, run::CodeInfo};

use super::{JudgeRun, Problem, TestCase};

#[derive(Responder)]
pub enum ProblemViewResponse {
    View(Template),
    NotFound(Status),
}

#[get("/")]
pub async fn list_problems_get(user: Option<&User>, mut db: DbConnection) -> Template {
    let problems = Problem::list(&mut db).await;
    Template::render("problems", context_with_base!(user, problems))
}

#[get("/<id>")]
pub async fn view_problem_get(
    user: Option<&User>,
    info: &State<CodeInfo>,
    mut db: DbConnection,
    id: i64,
) -> ProblemViewResponse {
    if let Some(problem) = Problem::get(&mut db, id).await {
        let last_run = if let Some(user) = user {
            JudgeRun::get_latest(&mut db, user.id, problem.id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let case_count = TestCase::count_for_problem(&mut db, problem.id)
            .await
            .unwrap_or(0);

        let code_info = &info.languages_json;
        let default_language = user
            .map(|u| &u.default_language)
            .filter(|l| info.run_config.languages.contains_key(*l))
            .unwrap_or(&info.run_config.default_language);

        ProblemViewResponse::View(Template::render(
            "problems/view",
            context_with_base!(
                user,
                problem,
                last_run,
                case_count,
                code_info,
                default_language
            ),
        ))
    } else {
        ProblemViewResponse::NotFound(Status::NotFound)
    }
}
