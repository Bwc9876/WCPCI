use chrono::NaiveDateTime;

use crate::db::DbPoolConnection;

pub struct ProblemCompletion {
    user_id: i64,
    problem_id: i64,
    completed_at: Option<NaiveDateTime>,
}

impl ProblemCompletion {
    /// Only insert is allowed as a problem completion is created when a user completes a problem for the
    /// first time __only__.
    pub async fn insert(&self, db: &mut DbPoolConnection) -> Result<(), sqlx::Error> {
        sqlx::query_as!(
            ProblemCompletion,
            "INSERT OR IGNORE INTO problem_completion (user_id, problem_id, completed_at) VALUES (?, ?, ?)",
            self.user_id,
            self.problem_id,
            self.completed_at
        )
        .execute(&mut **db)
        .await.map(|_| ())
    }

    pub async fn get_for_contest_and_user(
        db: &mut DbPoolConnection,
        contest_id: i64,
        user_id: i64,
    ) -> Vec<Self> {
        sqlx::query_as!(
            ProblemCompletion,
            "SELECT * FROM problem_completion WHERE user_id = ? AND problem_id IN (SELECT id FROM problem WHERE contest_id = ?)",
            user_id,
            contest_id
        )
        .fetch_all(&mut **db)
        .await
        .unwrap_or_default()
    }

    pub fn temp(user_id: i64, problem_id: i64) -> Self {
        Self {
            user_id,
            problem_id,
            completed_at: None,
        }
    }
}
