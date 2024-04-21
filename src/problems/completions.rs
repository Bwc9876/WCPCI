use chrono::NaiveDateTime;

use crate::db::DbPoolConnection;

#[derive(Serialize)]
pub struct ProblemCompletion {
    pub participant_id: i64,
    pub problem_id: i64,
    pub completed_at: Option<NaiveDateTime>,
    pub number_wrong: i64,
}

impl ProblemCompletion {
    pub async fn upsert(&self, db: &mut DbPoolConnection) -> Result<(), sqlx::Error> {
        sqlx::query_as!(
            ProblemCompletion,
            "INSERT OR REPLACE INTO problem_completion (participant_id, problem_id, completed_at, number_wrong) VALUES (?, ?, ?, ?)",
            self.participant_id,
            self.problem_id,
            self.completed_at,
            self.number_wrong
        )
        .execute(&mut **db)
        .await.map(|_| ())
    }

    pub async fn get_for_problem_and_participant(
        db: &mut DbPoolConnection,
        problem_id: i64,
        participant_id: i64,
    ) -> Option<Self> {
        sqlx::query_as!(
            ProblemCompletion,
            "SELECT * FROM problem_completion WHERE participant_id = ? AND problem_id = ?",
            participant_id,
            problem_id
        )
        .fetch_optional(&mut **db)
        .await
        .unwrap_or_default()
    }

    pub async fn get_for_participant(db: &mut DbPoolConnection, participant_id: i64) -> Vec<Self> {
        sqlx::query_as!(
            ProblemCompletion,
            "SELECT * FROM problem_completion WHERE participant_id = ?",
            participant_id,
        )
        .fetch_all(&mut **db)
        .await
        .unwrap_or_default()
    }

    pub fn temp(participant_id: i64, problem_id: i64, completed_at: Option<NaiveDateTime>) -> Self {
        Self {
            participant_id,
            problem_id,
            completed_at,
            number_wrong: 0,
        }
    }
}
