use std::sync::Arc;

use chrono::TimeZone;
use rocket::{fairing::AdHoc, get, http::Status, routes, State};

mod manager;
mod scoring;
mod ws;

pub use manager::{LeaderboardManager, LeaderboardManagerHandle};
use rocket_dyn_templates::Template;
use tokio::sync::Mutex;

use crate::{
    auth::users::{Admin, User},
    contests::{Contest, Participant},
    context_with_base,
    db::DbConnection,
    times::{datetime_to_html_time, ClientTimeZone},
};

use self::ws::leaderboard_ws;

#[derive(Responder)]
enum LeaderboardResponse {
    Template(Template),
    Error(Status),
}

#[derive(Serialize)]
struct ProblemIdTemp {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

#[get("/contests/<contest_id>/leaderboard")]
async fn leaderboard_get(
    mut db: DbConnection,
    leaderboard_manager: &State<LeaderboardManagerHandle>,
    contest_id: i64,
    tz: ClientTimeZone,
    user: Option<&User>,
    admin: Option<&Admin>,
) -> LeaderboardResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let mut leaderboard_manager = leaderboard_manager.lock().await;
        let leaderboard = leaderboard_manager
            .get_leaderboard(&mut db, &contest)
            .await
            .clone();
        drop(leaderboard_manager);
        let mut leaderboard = leaderboard.lock().await;

        let problems = sqlx::query_as!(
            ProblemIdTemp,
            "SELECT id, slug, name from problem WHERE contest_id = ?",
            contest.id
        )
        .fetch_all(&mut **db)
        .await
        .unwrap_or_default();

        let is_judge = if let Some(user) = user {
            Participant::get(&mut db, contest_id, user.id)
                .await
                .map(|p| p.is_judge)
                .unwrap_or(false)
        } else {
            false
        };

        let entries = leaderboard.full(&mut db).await;

        let start_local = tz.timezone().from_utc_datetime(&contest.start_time);
        let start_local_html = datetime_to_html_time(&start_local);
        let end_local = tz.timezone().from_utc_datetime(&contest.end_time);
        let end_local_html = datetime_to_html_time(&end_local);

        LeaderboardResponse::Template(Template::render(
            "contests/leaderboard",
            context_with_base!(user, freeze_percent: contest.freeze_percent(), progress: contest.progress(), has_started: contest.has_started(), start_local_html, end_local_html, is_running: contest.is_running(), contest, entries, problems, is_admin: admin.is_some(), is_judge),
        ))
    } else {
        LeaderboardResponse::Error(Status::NotFound)
    }
}

pub fn stage() -> AdHoc {
    let (tx, rx) = tokio::sync::watch::channel(false);

    AdHoc::on_ignite("Leaderboard App", |rocket| async {
        let shutdown_fairing = AdHoc::on_shutdown("Shutdown Leaderboard Sockets", |_rocket| {
            Box::pin(async move {
                tx.send(true).ok();
            })
        });

        let manager = LeaderboardManager::new(rx).await;
        rocket
            .attach(shutdown_fairing)
            .manage::<LeaderboardManagerHandle>(Arc::new(Mutex::new(manager)))
            .mount("/", routes![leaderboard_get, leaderboard_ws])
    })
}
