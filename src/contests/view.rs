use chrono::TimeZone;
use rocket::{get, http::Status};
use rocket_dyn_templates::Template;

use crate::{
    auth::users::{Admin, User},
    context_with_base,
    db::DbConnection,
    problems::{Problem, ProblemCompletion},
    times::{datetime_to_html_time, format_datetime_human_readable, ClientTimeZone},
};

use super::{Contest, Participant};

#[derive(Responder)]
pub enum ViewContestResponse {
    View(Template),
    Err(Status),
}

#[get("/<contest_id>")]
pub async fn view_contest(
    mut db: DbConnection,
    contest_id: i64,
    tz: ClientTimeZone,
    user: Option<&User>,
    admin: Option<&Admin>,
) -> ViewContestResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let participant = if let Some(user) = user {
            Participant::get(&mut db, contest_id, user.id).await
        } else {
            None
        };

        let problems = Problem::list(&mut db, contest_id).await;
        let problems_done = if let Some(participant) = &participant {
            ProblemCompletion::get_for_contest_and_user(&mut db, contest_id, participant.user_id)
                .await
                .len()
        } else {
            0
        };

        let (participants, judges) = Participant::list(&mut db, contest_id)
            .await
            .into_iter()
            .partition::<Vec<_>, _>(|p| !p.0.is_judge);

        let start_local = tz.timezone().from_utc_datetime(&contest.start_time);
        let start_local_html = datetime_to_html_time(&start_local);
        let end_local = tz.timezone().from_utc_datetime(&contest.end_time);

        let start_formatted = format_datetime_human_readable(start_local);
        let end_formatted = format_datetime_human_readable(end_local);
        let tz_name = tz.timezone().name();

        let ctx = context_with_base!(
            user,
            problems_done,
            problems_total: problems.len(),
            problems,
            participants,
            tz_name,
            start_formatted,
            start_local_html,
            end_formatted,
            is_admin: admin.is_some(),
            judges,
            started: contest.is_running(),
            contest,
            participant
        );
        ViewContestResponse::View(Template::render("contests/view", ctx))
    } else {
        ViewContestResponse::Err(Status::NotFound)
    }
}
