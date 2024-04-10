use std::sync::Arc;

use rocket::{fairing::AdHoc, get, http::Status, routes, State};

mod manager;
mod scoring;

pub use manager::{LeaderboardManager, LeaderboardManagerHandle};
use rocket_dyn_templates::Template;
use tokio::sync::Mutex;

use crate::{auth::users::User, contests::Contest, context_with_base, db::DbConnection};

#[derive(Responder)]
enum LeaderboardResponse {
    Template(Template),
    Error(Status),
}

struct ProblemIdTemp {
    pub id: i64,
}

#[get("/contests/<contest_id>/leaderboard")]
async fn leaderboard_get(
    mut db: DbConnection,
    leaderboard_manager: &State<LeaderboardManagerHandle>,
    contest_id: i64,
    user: Option<&User>,
) -> LeaderboardResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let mut leaderboard_manager = leaderboard_manager.lock().await;
        let leaderboard = leaderboard_manager
            .get_leaderboard(&mut db, &contest)
            .await
            .clone();
        drop(leaderboard_manager);
        let leaderboard = leaderboard.lock().await;

        let problem_ids = sqlx::query_as!(
            ProblemIdTemp,
            "SELECT id from problem WHERE contest_id = ?",
            contest.id
        )
        .fetch_all(&mut **db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>();

        let entries = leaderboard.full(&mut db).await;

        LeaderboardResponse::Template(Template::render(
            "contests/leaderboard",
            context_with_base!(user, contest, entries, problem_ids),
        ))
    } else {
        LeaderboardResponse::Error(Status::NotFound)
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Leaderboard App", |rocket| async {
        let manager = LeaderboardManager::new().await;
        rocket
            .manage::<LeaderboardManagerHandle>(Arc::new(Mutex::new(manager)))
            .mount("/", routes![leaderboard_get])
    })
}
