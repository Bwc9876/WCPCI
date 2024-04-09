use chrono::NaiveDateTime;

use crate::{
    contests::{Contest, Participant},
    db::DbPoolConnection,
    problems::ProblemCompletion,
};

pub struct ParticipantScores {
    contest_start: NaiveDateTime,
    contest_penalty_minutes: i64,
    pub participant_id: i64,
    pub user_id: i64,
    pub scores: Vec<(i64, i64)>, // id, score
}

impl ParticipantScores {
    async fn get_scores(
        db: &mut DbPoolConnection,
        id: i64,
        contest_start: NaiveDateTime,
        contest_penalty_minutes: i64,
    ) -> Vec<(i64, i64)> {
        let completions = ProblemCompletion::get_for_participant(db, id).await;
        completions
            .into_iter()
            .filter_map(|c| {
                c.completed_at.map(|d| {
                    (
                        c.problem_id,
                        (d - contest_start).num_seconds()
                            + (c.number_wrong * contest_penalty_minutes * 60),
                    )
                })
            })
            .collect::<Vec<_>>()
    }

    pub async fn new(
        db: &mut DbPoolConnection,
        participant: &Participant,
        contest: &Contest,
    ) -> Self {
        Self {
            contest_start: contest.start_time,
            contest_penalty_minutes: contest.penalty,
            participant_id: participant.p_id,
            user_id: participant.user_id,
            scores: Self::get_scores(db, participant.p_id, contest.start_time, contest.penalty)
                .await,
        }
    }

    pub async fn refresh(&mut self, db: &mut DbPoolConnection) {
        self.scores = Self::get_scores(
            db,
            self.participant_id,
            self.contest_start,
            self.contest_penalty_minutes,
        )
        .await;
    }

    pub async fn update_contest(&mut self, db: &mut DbPoolConnection, contest: &Contest) {
        self.contest_start = contest.start_time;
        self.contest_penalty_minutes = contest.penalty;
        self.refresh(db).await;
    }
}

impl Eq for ParticipantScores {}

impl PartialEq for ParticipantScores {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl PartialOrd for ParticipantScores {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ParticipantScores {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.scores
            .len()
            .cmp(&other.scores.len())
            .then(
                self.scores
                    .iter()
                    .map(|(_, score)| score)
                    .sum::<i64>()
                    .cmp(&other.scores.iter().map(|(_, score)| score).sum()),
            )
            .reverse()
    }
}
