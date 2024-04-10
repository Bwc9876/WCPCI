use chrono::NaiveDateTime;
use rocket::get;
use rocket::http::Status;
use rocket_dyn_templates::Template;

use crate::auth::users::User;
use crate::contests::Contest;
use crate::context_with_base;
use crate::db::{DbConnection, DbPoolConnection};
use crate::run::JobState;

use super::Problem;

#[derive(Serialize)]
pub struct JudgeRun {
    pub id: i64,
    pub problem_id: i64,
    pub participant_id: i64,
    pub amount_run: i64,
    pub total_cases: i64,
    pub error: Option<String>,
    #[serde(serialize_with = "crate::times::serialize_to_js")]
    pub ran_at: NaiveDateTime,
}

impl JudgeRun {
    pub fn temp(
        problem_id: i64,
        participant_id: i64,
        amount_run: i64,
        total_cases: i64,
        error: Option<String>,
        ran_at: NaiveDateTime,
    ) -> Self {
        Self {
            id: 0,
            problem_id,
            participant_id,
            amount_run,
            total_cases,
            error,
            ran_at,
        }
    }

    pub fn from_job_state(
        problem_id: i64,
        participant_id: i64,
        state: &JobState,
        ran_at: NaiveDateTime,
    ) -> Self {
        let (amount_run, _, error) = state.last_error();
        Self::temp(
            problem_id,
            participant_id,
            amount_run as i64,
            state.len() as i64,
            error,
            ran_at,
        )
    }

    pub async fn list(
        db: &mut DbPoolConnection,
        participant_id: i64,
        problem_id: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            JudgeRun,
            "SELECT * FROM judge_run WHERE participant_id = ? AND problem_id = ? ORDER BY ran_at DESC LIMIT 10",
            participant_id,
            problem_id
        )
        .fetch_all(&mut **db)
        .await
    }

    pub async fn get_latest(
        db: &mut DbPoolConnection,
        participant_id: i64,
        problem_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            JudgeRun,
            "SELECT * FROM judge_run WHERE participant_id = ? AND problem_id = ? ORDER BY ran_at DESC LIMIT 1",
            participant_id,
            problem_id
        )
            .fetch_optional(&mut **db)
            .await
    }

    pub async fn write_to_db(self, db: &mut DbPoolConnection) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            JudgeRun,
            "INSERT INTO judge_run (problem_id, participant_id, amount_run, total_cases, error, ran_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
            self.problem_id,
            self.participant_id,
            self.amount_run,
            self.total_cases,
            self.error,
            self.ran_at
        )
            .fetch_one(&mut **db)
            .await
    }

    pub fn success(&self) -> bool {
        self.amount_run == self.total_cases && self.error.is_none()
    }
}

#[derive(Responder)]
pub enum RunsResponse {
    NotFound(Status),
    Ok(Template),
}

#[get("/<contest_id>/problems/<slug>/runs")]
pub async fn runs(
    contest_id: i64,
    slug: String,
    user: Option<&User>,
    mut db: DbConnection,
) -> RunsResponse {
    if let Some(problem) = Problem::get(&mut db, contest_id, &slug).await {
        let contest = Contest::get(&mut db, contest_id).await.unwrap();
        let runs = if let Some(user) = user {
            JudgeRun::list(&mut db, user.id, problem.id)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };
        RunsResponse::Ok(Template::render(
            "problems/runs",
            context_with_base!(user, runs, contest, problem),
        ))
    } else {
        RunsResponse::NotFound(Status::NotFound)
    }
}
