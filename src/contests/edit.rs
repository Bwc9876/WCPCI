use chrono::TimeZone;
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

use crate::leaderboard::LeaderboardManagerHandle;
use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::User,
    },
    context_with_base_authed,
    db::DbConnection,
    template::FormTemplateObject,
    times::ClientTimeZone,
};

use super::{Contest, ContestForm, ContestFormTemplate, Participant};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum EditContestResponse {
    Form(Template),
    Redirect(Redirect),
    Error(Status),
}

#[get("/<id>/edit")]
pub async fn edit_contest_get(
    user: &User,
    mut db: DbConnection,
    id: i64,
    tz: ClientTimeZone,
    _token: &CsrfToken,
) -> EditContestResponse {
    if let Some(contest) = Contest::get(&mut db, id).await {
        let all_users = User::list(&mut db).await.unwrap_or_default();
        let judges = Participant::list_judge(&mut db, contest.id).await;
        let form_template = ContestFormTemplate {
            contest: Some(&contest),
            judges: &judges,
            timezone: &tz,
        };
        let form = FormTemplateObject::get(form_template);
        EditContestResponse::Form(Template::render(
            "contests/edit",
            context_with_base_authed!(user, form, judges, all_users, contest),
        ))
    } else {
        EditContestResponse::Error(Status::NotFound)
    }
}

#[post("/<id>/edit", data = "<form>")]
pub async fn edit_contest_post(
    id: i64,
    user: &User,
    form: Form<Contextual<'_, ContestForm<'_>>>,
    leaderboard_handle: &State<LeaderboardManagerHandle>,
    client_time_zone: ClientTimeZone,
    _token: &VerifyCsrfToken,
    mut db: DbConnection,
) -> EditContestResponse {
    if let Some(mut contest) = Contest::get(&mut db, id).await {
        if let Some(ref value) = form.value {
            let tz = client_time_zone.timezone();
            let original_start_time = contest.start_time;
            let original_penalty = contest.penalty;
            contest.name = value.name.to_string();
            contest.description = value.description.map(|s| s.to_string());
            contest.start_time = tz
                .from_local_datetime(&value.start_time.0)
                .unwrap()
                .naive_utc();
            contest.registration_deadline = tz
                .from_local_datetime(&value.registration_deadline.0)
                .unwrap()
                .naive_utc();
            contest.end_time = tz
                .from_local_datetime(&value.end_time.0)
                .unwrap()
                .naive_utc();
            contest.max_participants = value.max_participants;
            contest.penalty = value.penalty;
            contest.freeze_time = value.freeze_time;

            if let Err(why) = contest.update(&mut db).await {
                error!("Failed to insert contest: {}", why);
                EditContestResponse::Error(Status::InternalServerError)
            } else {
                let participants = Participant::list(&mut db, contest.id).await;
                let mut visited: Vec<i64> = vec![];
                for (participant, _) in participants {
                    visited.push(participant.user_id);
                    // if participant is a judge and is not in the list of new judges, delete them
                    if participant.is_judge
                        && !(value
                            .judges
                            .get(&participant.user_id)
                            .copied()
                            .unwrap_or(false))
                    {
                        Participant::remove(&mut db, contest.id, participant.user_id)
                            .await
                            .unwrap();
                    }
                }
                for judge in value.judges.keys().filter(|k| !visited.contains(k)) {
                    let res = Participant::create_or_make_judge(&mut db, contest.id, *judge).await;
                    if let Err(why) = res {
                        error!("Failed to insert judge: {}", why);
                    }
                }

                if contest.start_time != original_start_time || contest.penalty != original_penalty
                {
                    let mut leaderboard_manager = leaderboard_handle.lock().await;
                    let leaderboard = leaderboard_manager.get_leaderboard(&mut db, &contest).await;
                    drop(leaderboard_manager);
                    let mut leaderboard = leaderboard.lock().await;
                    leaderboard.full_refresh(&mut db, Some(&contest)).await;
                }

                EditContestResponse::Redirect(Redirect::to("/contests"))
            }
        } else {
            let all_users = User::list(&mut db).await.unwrap_or_default();
            let judges = Participant::list_judge(&mut db, contest.id).await;
            let form_template = ContestFormTemplate {
                contest: None,
                judges: &judges,
                timezone: &client_time_zone,
            };
            let form = FormTemplateObject::from_rocket_context(form_template, &form.context);
            let ctx = context_with_base_authed!(user, form, judges, all_users, contest);
            EditContestResponse::Form(Template::render("contests/edit", ctx))
        }
    } else {
        EditContestResponse::Error(Status::NotFound)
    }
}
