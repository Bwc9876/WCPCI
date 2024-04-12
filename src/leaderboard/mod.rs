use std::sync::Arc;

use rocket::{fairing::AdHoc, get, http::Status, routes, State};

mod manager;
mod scoring;

pub use manager::{LeaderboardManager, LeaderboardManagerHandle};
use rocket_dyn_templates::Template;
use tokio::sync::Mutex;

use crate::{
    auth::users::{Admin, User},
    contests::{Contest, Participant},
    context_with_base,
    db::DbConnection,
};

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
        let leaderboard = leaderboard.lock().await;

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

        LeaderboardResponse::Template(Template::render(
            "contests/leaderboard",
            context_with_base!(user, has_started: contest.has_started(), contest, entries, problems, is_admin: admin.is_some(), is_judge),
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
