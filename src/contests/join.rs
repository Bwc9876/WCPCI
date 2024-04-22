use log::error;
use rocket::{
    get,
    http::Status,
    request::{FromRequest, Outcome},
    response::Redirect,
    State,
};

use crate::{
    auth::users::{Admin, User},
    db::DbConnection,
    leaderboard::LeaderboardManagerHandle,
};

use super::{Contest, Participant};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum JoinContestResponse {
    Redirect(Redirect),
    Err(Status),
}

pub struct NoPrefetch;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NoPrefetch {
    type Error = ();

    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let headers = req.headers();
        let a = headers
            .get_one("X-Purpose")
            .map_or(false, |s| s == "Preview");
        let b = headers.get_one("X-moz").map_or(false, |s| s == "prefetch");
        if a || b {
            Outcome::Forward(Status::Forbidden)
        } else {
            Outcome::Success(NoPrefetch)
        }
    }
}

#[get("/<contest_id>/join", rank = 10)]
pub async fn join_contest(
    mut db: DbConnection,
    contest_id: i64,
    leaderboard_handle: &State<LeaderboardManagerHandle>,
    _no_prefetch: NoPrefetch,
    user: &User,
    admin: Option<&Admin>,
) -> JoinContestResponse {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        if admin.is_some()
            || Participant::get(&mut db, contest_id, user.id)
                .await
                .is_some()
        {
            JoinContestResponse::Redirect(Redirect::to(format!("/contests/{}/", contest_id)))
        } else if contest.can_register() {
            if let Some(max_participants) = &contest.max_participants {
                let participants = Participant::list_not_judge(&mut db, contest_id).await;
                if participants.len() >= *max_participants as usize {
                    return JoinContestResponse::Err(Status::Forbidden);
                }
            }
            let participant = Participant::temp(user.id, contest_id, false);
            if let Err(why) = participant.insert(&mut db).await {
                error!("Error inserting participant: {:?}", why);
                JoinContestResponse::Err(Status::InternalServerError)
            } else {
                let mut leaderboard_manager = leaderboard_handle.lock().await;
                leaderboard_manager
                    .refresh_leaderboard(&mut db, &contest)
                    .await;

                JoinContestResponse::Redirect(Redirect::to(format!("/contests/{}/", contest_id)))
            }
        } else {
            JoinContestResponse::Err(Status::Forbidden)
        }
    } else {
        JoinContestResponse::Err(Status::NotFound)
    }
}
