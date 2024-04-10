use rocket::{get, http::Status, State};
use rocket_dyn_templates::Template;

use crate::{
    auth::users::{Admin, User},
    contests::{Contest, Participant},
    context_with_base,
    db::DbConnection,
    run::CodeInfo,
};

use super::{JudgeRun, Problem, TestCase};

#[derive(Responder)]
pub enum ProblemViewResponse {
    View(Template),
    NotFound(Status),
}

#[get("/<contest_id>/problems")]
pub async fn list_problems_get(
    user: Option<&User>,
    admin: Option<&Admin>,
    contest_id: i64,
    mut db: DbConnection,
) -> ProblemViewResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let is_judge = if let Some(user) = user {
            Participant::get(&mut db, contest_id, user.id)
                .await
                .map(|p| p.is_judge)
                .unwrap_or(false)
        } else {
            false
        };
        let is_admin = admin.is_some();
        if contest.has_started() || is_judge || is_admin {
            let problems = Problem::list(&mut db, contest_id).await;
            ProblemViewResponse::View(Template::render(
                "problems",
                context_with_base!(user, problems, is_admin, contest, can_edit: is_judge || is_admin),
            ))
        } else {
            ProblemViewResponse::NotFound(Status::NotFound)
        }
    } else {
        ProblemViewResponse::NotFound(Status::NotFound)
    }
}

#[get("/<contest_id>/problems/<slug>", rank = 10)]
pub async fn view_problem_get(
    user: Option<&User>,
    admin: Option<&Admin>,
    info: &State<CodeInfo>,
    mut db: DbConnection,
    contest_id: i64,
    slug: &str,
) -> ProblemViewResponse {
    if let Some(problem) = Problem::get(&mut db, contest_id, slug).await {
        let is_judge = if let Some(user) = user {
            Participant::get(&mut db, contest_id, user.id)
                .await
                .map(|p| p.is_judge)
                .unwrap_or(false)
        } else {
            false
        };
        let is_admin = admin.is_some();

        let contest = Contest::get(&mut db, contest_id).await.unwrap();
        if !contest.has_started() && !is_judge && !is_admin {
            return ProblemViewResponse::NotFound(Status::NotFound);
        }

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

        let languages = info.run_config.get_languages_for_dropdown();
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
                contest,
                code_info,
                languages,
                default_language,
                can_edit: is_judge || is_admin
            ),
        ))
    } else {
        ProblemViewResponse::NotFound(Status::NotFound)
    }
}
