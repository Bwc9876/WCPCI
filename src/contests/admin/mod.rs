use rocket::{fairing::AdHoc, get, http::Status, routes};
use rocket_dyn_templates::Template;

use crate::{
    auth::users::{Admin, User},
    context_with_base_authed,
    db::DbConnection,
};

use super::{Contest, Participant};

mod completions;
mod participants;
mod runs;

#[get("/contests/<contest_id>/admin")]
async fn contest_admin(
    mut db: DbConnection,
    contest_id: i64,
    user: &User,
    admin: Option<&Admin>,
) -> Result<Template, Status> {
    if let Some(contest) = Contest::get(&mut db, contest_id).await {
        let participant = Participant::get(&mut db, contest_id, user.id).await;
        let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
        if allowed {
            let ctx = context_with_base_authed!(user, contest);
            Ok(Template::render("contests/admin", ctx))
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::NotFound)
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Contest Admin", |rocket| async {
        rocket.mount(
            "/",
            routes![
                contest_admin,
                participants::participants,
                participants::kick_participant_get,
                participants::kick_participant_post,
                runs::runs,
                runs::cancel,
                runs::cancel_post,
                runs::problem,
                completions::edit_completion,
                completions::edit_completion_post,
            ],
        )
    })
}