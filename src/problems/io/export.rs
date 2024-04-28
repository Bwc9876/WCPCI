use rocket::{get, http::Status, serde::json::Json};

use crate::{
    auth::users::{Admin, User},
    contests::Participant,
    db::DbConnection,
    error::prelude::*,
    problems::Problem,
};

use super::ProblemData;

#[get("/contests/<contest_id>/problems/<problem_slug>/export")]
pub async fn problem_export(
    mut db: DbConnection,
    contest_id: i64,
    admin: Option<&Admin>,
    user: &User,
    problem_slug: &str,
) -> ResultResponse<Json<ProblemData>> {
    let problem = Problem::get_or_404(&mut db, contest_id, problem_slug).await?;
    let participant = Participant::get(&mut db, contest_id, user.id).await?;
    let allowed = admin.is_some() || participant.map_or(false, |p| p.is_judge);
    if allowed {
        let data = ProblemData::get_for_problem(&mut db, &problem)
            .await
            .with_context(|| {
                format!("Couldn't export problem {problem_slug} from contest {contest_id}")
            })?;
        Ok(Json(data))
    } else {
        Err(Status::Forbidden.into())
    }
}
