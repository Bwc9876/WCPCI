use rocket::{get, http::Status, response::Redirect};

use crate::{
    auth::users::{Admin, User},
    db::DbConnection,
};

use super::{Contest, Participant};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum JoinContestResponse {
    Redirect(Redirect),
    Err(Status),
}

#[get("/<contest_id>/join", rank = 10)]
pub async fn join_contest(
    mut db: DbConnection,
    contest_id: i64,
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
            participant.insert(&mut db).await.ok();
            JoinContestResponse::Redirect(Redirect::to(format!("/contests/{}/", contest_id)))
        } else {
            JoinContestResponse::Err(Status::Forbidden)
        }
    } else {
        JoinContestResponse::Err(Status::NotFound)
    }
}
