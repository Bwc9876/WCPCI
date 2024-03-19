use rocket::get;
use rocket::http::Status;
use rocket::time::OffsetDateTime;
use rocket_dyn_templates::Template;

use crate::auth::users::User;
use crate::context_with_base_authed;
use crate::db::{DbConnection, DbPoolConnection};
use crate::run::JobState;

use super::Problem;

#[derive(Serialize)]
pub struct JudgeRun {
    pub id: i64,
    pub problem_id: i64,
    pub user_id: i64,
    pub amount_run: i64,
    pub total_cases: i64,
    pub error: Option<String>,
    #[serde(skip)]
    pub ran_at: OffsetDateTime,
}

impl JudgeRun {
    pub fn temp(
        problem_id: i64,
        user_id: i64,
        amount_run: i64,
        total_cases: i64,
        error: Option<String>,
        ran_at: OffsetDateTime,
    ) -> Self {
        Self {
            id: 0,
            problem_id,
            user_id,
            amount_run,
            total_cases,
            error,
            ran_at,
        }
    }

    pub fn from_job_state(
        problem_id: i64,
        user_id: i64,
        state: JobState,
        ran_at: OffsetDateTime,
    ) -> Self {
        let (amount_run, error) = state.last_error();
        Self::temp(
            problem_id,
            user_id,
            amount_run as i64,
            state.len() as i64,
            error,
            ran_at,
        )
    }

    pub async fn list(
        db: &mut DbPoolConnection,
        user_id: i64,
        problem_id: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            JudgeRun,
            "SELECT * FROM judge_run WHERE user_id = ? AND problem_id = ? ORDER BY ran_at DESC LIMIT 10",
            user_id,
            problem_id
        )
        .fetch_all(&mut **db)
        .await
    }

    pub async fn get_latest(
        db: &mut DbPoolConnection,
        user_id: i64,
        problem_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            JudgeRun,
            "SELECT * FROM judge_run WHERE user_id = ? AND problem_id = ? ORDER BY ran_at DESC LIMIT 1",
            user_id,
            problem_id
        )
            .fetch_optional(&mut **db)
            .await
    }

    pub async fn write_to_db(self, db: &mut DbPoolConnection) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            JudgeRun,
            "INSERT INTO judge_run (problem_id, user_id, amount_run, total_cases, error, ran_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
            self.problem_id,
            self.user_id,
            self.amount_run,
            self.total_cases,
            self.error,
            self.ran_at
        )
            .fetch_one(&mut **db)
            .await
    }
}

#[derive(Responder)]
pub enum RunsResponse {
    NotFound(Status),
    Ok(Template),
}

#[get("/<id>/runs")]
pub async fn runs(id: i64, user: &User, mut db: DbConnection) -> RunsResponse {
    if let Some(problem) = Problem::get(&mut db, id).await {
        let runs = JudgeRun::list(&mut db, user.id, problem.id).await.unwrap();
        RunsResponse::Ok(Template::render(
            "problems/runs",
            context_with_base_authed!(user, runs, problem),
        ))
    } else {
        RunsResponse::NotFound(Status::NotFound)
    }
}
