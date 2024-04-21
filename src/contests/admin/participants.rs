use rocket::{get, http::Status, post, response::Redirect};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::{Admin, User},
    },
    contests::{Contest, Participant},
    context_with_base_authed,
    db::DbConnection,
};

#[derive(Serialize, Debug)]
struct Row {
    participant: Participant,
    user: User,
}

#[get("/contests/<contest_id>/admin/participants")]
pub async fn participants(
    mut db: DbConnection,
    user: &User,
    admin: Option<&Admin>,
    contest_id: i64,
) -> Result<Template, Status> {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let participant = Participant::get(&mut db, contest_id, user.id).await;
        let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
        if allowed {
            let just_participants = Participant::list_not_judge(&mut db, contest_id).await;
            let mut participants = vec![];
            for participant in just_participants {
                let p_user = User::get(&mut db, participant.user_id).await;
                if let Some(user) = p_user {
                    participants.push(Row { participant, user })
                }
            }
            let ctx = context_with_base_authed!(user, contest, participants);
            Ok(Template::render("contests/admin/participants", ctx))
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::NotFound)
    }
}

#[get("/contests/<contest_id>/admin/participants/<p_id>/kick")]
pub async fn kick_participant_get(
    contest_id: i64,
    p_id: i64,
    mut db: DbConnection,
    user: &User,
    _token: &CsrfToken,
    admin: Option<&Admin>,
) -> Result<Template, Status> {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let participant = Participant::get(&mut db, contest_id, user.id).await;
        let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
        if allowed {
            let target_participant = Participant::get(&mut db, contest_id, p_id)
                .await
                .ok_or(Status::NotFound)?;
            let target_user = User::get(&mut db, target_participant.user_id)
                .await
                .ok_or(Status::NotFound)?;
            let ctx = context_with_base_authed!(user, contest, target_participant, target_user);
            Ok(Template::render("contests/admin/kick", ctx))
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::NotFound)
    }
}

#[post("/contests/<contest_id>/admin/participants/<p_id>/kick")]
pub async fn kick_participant_post(
    contest_id: i64,
    p_id: i64,
    mut db: DbConnection,
    user: &User,
    _token: &VerifyCsrfToken,
    admin: Option<&Admin>,
) -> Result<Redirect, Status> {
    if let Some(_contest) = Contest::get(&mut db, contest_id).await {
        let participant = Participant::get(&mut db, contest_id, user.id).await;
        let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
        if allowed {
            let target_participant = Participant::get(&mut db, contest_id, p_id)
                .await
                .ok_or(Status::NotFound)?;
            target_participant.delete(&mut db).await.map_err(|e| {
                log::error!("Failed to delete participant: {:?}", e);
                Status::InternalServerError
            })?;
            Ok(Redirect::to(format!(
                "/contests/{}/admin/participants",
                contest_id
            )))
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::NotFound)
    }
}
