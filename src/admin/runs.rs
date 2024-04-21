use log::error;
use rocket::{get, http::Status, post, response::Redirect, State};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::{Admin, User},
    },
    contests::Contest,
    context_with_base_authed,
    db::DbConnection,
    run::ManagerHandle,
};

#[derive(Serialize)]
struct TempProblem {
    id: i64,
    slug: String,
    contest_id: i64,
}

#[derive(Serialize)]
pub struct RunsAdminRow {
    user: User,
    problem: TempProblem,
}

#[get("/runs")]
pub async fn runs(
    mut db: DbConnection,
    user: &User,
    _admin: &Admin,
    manager_handle: &State<ManagerHandle>,
) -> Result<Template, Status> {
    let manager = manager_handle.lock().await;
    let jobs = manager.all_active_jobs().await;
    drop(manager);
    let mut rows = Vec::with_capacity(jobs.len());
    for (job_user_id, problem_id) in jobs {
        let job_user = User::get(&mut db, job_user_id).await.ok_or_else(|| {
            error!("Couldn't find user with id {}", job_user_id);
            Status::InternalServerError
        })?;
        let problem = sqlx::query_as!(
            TempProblem,
            "SELECT id, slug, contest_id FROM problem WHERE id = ?",
            problem_id
        )
        .fetch_one(&mut **db)
        .await
        .map_err(|e| {
            error!("Couldn't find problem with id {}: {:?}", problem_id, e);
            Status::InternalServerError
        })?;
        rows.push(RunsAdminRow {
            user: job_user,
            problem,
        });
    }

    let contests = Contest::list(&mut db).await;

    let ctx = context_with_base_authed!(user, rows, contests);
    Ok(Template::render("admin/runs", ctx))
}

#[get("/runs/<user_id>/<problem_id>/cancel")]
pub async fn cancel_run(
    mut db: DbConnection,
    user_id: i64,
    problem_id: i64,
    user: &User,
    _admin: &Admin,
    _token: &CsrfToken,
    manager_handle: &State<ManagerHandle>,
) -> Result<Template, Status> {
    let manager = manager_handle.lock().await;
    if manager.get_handle(user_id, problem_id).await.is_some() {
        let target_user = User::get(&mut db, user_id).await.ok_or(Status::NotFound)?;
        Ok(Template::render(
            "admin/runs_cancel",
            context_with_base_authed!(user, target_user, problem_id),
        ))
    } else {
        Err(Status::NotFound)
    }
}

#[post("/runs/<user_id>/<problem_id>/cancel")]
pub async fn cancel_run_post(
    user_id: i64,
    problem_id: i64,
    _user: &User,
    _admin: &Admin,
    _token: &VerifyCsrfToken,
    manager_handle: &State<ManagerHandle>,
) -> Result<Redirect, Status> {
    let mut manager = manager_handle.lock().await;
    if manager.get_handle(user_id, problem_id).await.is_some() {
        manager.shutdown_job(user_id).await;
        Ok(Redirect::to("/admin/runs"))
    } else {
        Err(Status::NotFound)
    }
}

#[get("/runs/cancel-all")]
pub async fn cancel_all_runs(
    user: &User,
    _admin: &Admin,
    _token: &CsrfToken,
) -> Result<Template, Status> {
    Ok(Template::render(
        "admin/runs_cancel_all",
        context_with_base_authed!(user,),
    ))
}

#[post("/runs/cancel-all")]
pub async fn cancel_all_runs_post(
    _user: &User,
    _admin: &Admin,
    _token: &VerifyCsrfToken,
    manager_handle: &State<ManagerHandle>,
) -> Result<Redirect, Status> {
    let mut manager = manager_handle.lock().await;
    manager.shutdown().await;
    Ok(Redirect::to("/admin/runs"))
}
