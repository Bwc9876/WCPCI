use std::collections::HashMap;

use anyhow::Context;
use log::error;
use rocket::{
    form::{Contextual, Form},
    get,
    http::Status,
    post,
    response::Redirect,
    FromForm, State,
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
    leaderboard::LeaderboardManagerHandle,
    problems::{Problem, ProblemCompletion},
    template::{FormTemplateObject, TemplatedForm},
    FormResponse, ResultResponse,
};

struct CompletionTemplateForm<'r> {
    completion: Option<&'r ProblemCompletion>,
    contest: &'r Contest,
}

impl<'r> TemplatedForm for CompletionTemplateForm<'r> {
    fn get_defaults(&mut self) -> HashMap<String, String> {
        if let Some(completion) = &self.completion {
            let diff = completion
                .completed_at
                .map(|c| {
                    c.signed_duration_since(self.contest.start_time)
                        .num_minutes()
                        .to_string()
                })
                .unwrap_or_else(String::new);
            HashMap::from_iter([
                ("completed_in".to_string(), diff),
                (
                    "number_wrong".to_string(),
                    completion.number_wrong.to_string(),
                ),
            ])
        } else {
            HashMap::from_iter([
                ("completed_in".to_string(), "".to_string()),
                ("number_wrong".to_string(), "0".to_string()),
            ])
        }
    }
}

#[get("/contests/<contest_id>/admin/runs/problems/<problem_slug>/edit/<participant_id>")]
pub async fn edit_completion(
    mut db: DbConnection,
    participant_id: i64,
    contest_id: i64,
    problem_slug: &str,
    _token: &CsrfToken,
    user: &User,
    admin: Option<&Admin>,
) -> ResultResponse<Template> {
    let contest = Contest::get_or_404(&mut db, contest_id).await?;
    let problem = Problem::get_or_404(&mut db, contest_id, problem_slug).await?;
    let target_participant = Participant::by_id(&mut db, participant_id)
        .await
        .context("Failed to get participant")?
        .ok_or(Status::NotFound)?;
    let target_user = User::get(&mut db, target_participant.user_id)
        .await?
        .ok_or(Status::NotFound)?;

    let participant = Participant::get(&mut db, contest_id, user.id).await?;
    let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
    if allowed {
        let completion =
            ProblemCompletion::get_for_problem_and_participant(&mut db, problem.id, participant_id)
                .await?;
        let form = CompletionTemplateForm {
            completion: completion.as_ref(),
            contest: &contest,
        };
        let form = FormTemplateObject::get(form);
        let ctx = context_with_base_authed!(
            user,
            contest,
            target_participant,
            target_user,
            problem,
            form
        );
        Ok(Template::render("contests/admin/runs_completion", ctx))
    } else {
        Err(Status::Forbidden.into())
    }
}

#[inline]
fn over_0<'e>(value: &Option<i64>) -> Result<(), rocket::form::Errors<'e>> {
    if let Some(i) = value {
        if *i > 0 {
            Ok(())
        } else {
            let err = rocket::form::Error::validation("Must be over 0");
            Err(err.into())
        }
    } else {
        Ok(())
    }
}

#[derive(FromForm)]
pub struct ProblemCompletionForm {
    #[field(validate = over_0())]
    completed_in: Option<i64>,
    #[field(validate = range(0..))]
    number_wrong: i64,
}

#[post(
    "/contests/<contest_id>/admin/runs/problems/<problem_slug>/edit/<participant_id>",
    data = "<form>"
)]
#[allow(clippy::too_many_arguments)]
pub async fn edit_completion_post(
    mut db: DbConnection,
    participant_id: i64,
    contest_id: i64,
    problem_slug: &str,
    _token: &VerifyCsrfToken,
    leaderboard_manager: &State<LeaderboardManagerHandle>,
    form: Form<Contextual<'_, ProblemCompletionForm>>,
    user: &User,
    admin: Option<&Admin>,
) -> FormResponse {
    let contest = Contest::get_or_404(&mut db, contest_id).await?;
    let problem = Problem::get_or_404(&mut db, contest_id, problem_slug).await?;
    let target_participant = Participant::by_id(&mut db, participant_id)
        .await
        .context("Failed to get participant")?
        .ok_or(Status::NotFound)?;
    let target_user = User::get_or_404(&mut db, target_participant.user_id).await?;

    let participant = Participant::get(&mut db, contest_id, user.id).await?;
    let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
    if allowed {
        let completion =
            ProblemCompletion::get_for_problem_and_participant(&mut db, problem.id, participant_id)
                .await?;
        if let Some(ref value) = form.value {
            let completed_at = value
                .completed_in
                .map(|c| contest.start_time + chrono::Duration::minutes(c));
            let number_wrong = value.number_wrong;
            let completion = ProblemCompletion {
                participant_id,
                problem_id: problem.id,
                completed_at,
                number_wrong,
            };
            completion.upsert(&mut db).await.map_err(|e| {
                error!("Failed to upsert completion: {}", e);
                Status::InternalServerError
            })?;
            let mut leaderboard_manager = leaderboard_manager.lock().await;
            leaderboard_manager
                .process_completion(&completion, &contest)
                .await;
            return Ok(Redirect::to(format!(
                "/contests/{}/admin/runs/problems/{}",
                contest_id, problem_slug
            )));
        }
        let form_template = CompletionTemplateForm {
            completion: completion.as_ref(),
            contest: &contest,
        };
        let form = FormTemplateObject::from_rocket_context(form_template, &form.context);
        let ctx = context_with_base_authed!(
            user,
            target_participant,
            target_user,
            contest,
            problem,
            form
        );
        Err(Template::render("contests/admin/runs_completion", ctx).into())
    } else {
        Err(Status::Forbidden.into())
    }
}
