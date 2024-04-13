use chrono::NaiveDateTime;
use chrono::TimeZone;
use rocket::get;
use rocket::http::Status;
use rocket_dyn_templates::Template;

use crate::auth::users::User;
use crate::contests::Contest;
use crate::context_with_base;
use crate::db::{DbConnection, DbPoolConnection};
use crate::run::JobState;
use crate::times::format_datetime_human_readable;
use crate::times::ClientTimeZone;

use super::Problem;

#[derive(Serialize)]
pub struct JudgeRun {
    pub id: i64,
    pub problem_id: i64,
    pub user_id: i64,
    pub amount_run: i64,
    pub program: String,
    pub language: String,
    pub total_cases: i64,
    pub error: Option<String>,
    #[serde(serialize_with = "crate::times::serialize_to_js")]
    pub ran_at: NaiveDateTime,
}

impl JudgeRun {
    #[allow(clippy::too_many_arguments)]
    pub fn temp(
        problem_id: i64,
        user_id: i64,
        amount_run: i64,
        program: String,
        language: String,
        total_cases: i64,
        error: Option<String>,
        ran_at: NaiveDateTime,
    ) -> Self {
        Self {
            id: 0,
            problem_id,
            user_id,
            amount_run,
            program,
            language,
            total_cases,
            error,
            ran_at,
        }
    }

    pub fn from_job_state(
        problem_id: i64,
        user_id: i64,
        program: String,
        language: String,
        state: &JobState,
        ran_at: NaiveDateTime,
    ) -> Self {
        let (amount_run, _, error) = state.last_error();
        Self::temp(
            problem_id,
            user_id,
            amount_run as i64,
            program,
            language,
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

    pub const MAX_RUNS_PER_USER: i64 = 25;

    pub async fn write_to_db(self, db: &mut DbPoolConnection) -> Result<Self, sqlx::Error> {
        let new = sqlx::query_as!(
            JudgeRun,
            "INSERT INTO judge_run (problem_id, user_id, amount_run, program, language, total_cases, error, ran_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
            self.problem_id,
            self.user_id,
            self.amount_run,
            self.program,
            self.language,
            self.total_cases,
            self.error,
            self.ran_at
        )
            .fetch_one(&mut **db)
            .await;

        let run_count = sqlx::query!(
            "SELECT * FROM judge_run WHERE user_id = ? AND problem_id = ?",
            self.user_id,
            self.problem_id
        )
        .fetch_all(&mut **db)
        .await?
        .len() as i64;

        if run_count > Self::MAX_RUNS_PER_USER {
            sqlx::query!(
                "DELETE FROM judge_run WHERE id = (SELECT id FROM judge_run WHERE user_id = ? AND problem_id = ? ORDER BY ran_at ASC LIMIT 1)",
                self.user_id,
                self.problem_id
            )
                .execute(&mut **db)
                .await?;
        }

        new
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
    slug: &str,
    tz: ClientTimeZone,
    user: Option<&User>,
    mut db: DbConnection,
) -> RunsResponse {
    if let Some(problem) = Problem::get(&mut db, contest_id, slug).await {
        let contest = Contest::get(&mut db, contest_id).await.unwrap();
        let runs = if let Some(user) = user {
            JudgeRun::list(&mut db, user.id, problem.id)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };
        let tz = tz.timezone();
        let formatted_times = runs
            .iter()
            .map(|r| tz.from_utc_datetime(&r.ran_at))
            .map(format_datetime_human_readable)
            .collect::<Vec<_>>();
        RunsResponse::Ok(Template::render(
            "problems/runs",
            context_with_base!(user, runs, contest, problem, formatted_times, max_runs: JudgeRun::MAX_RUNS_PER_USER),
        ))
    } else {
        RunsResponse::NotFound(Status::NotFound)
    }
}
