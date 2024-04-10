use std::{collections::HashMap, sync::Arc};

use sqlx::FromRow;
use tokio::sync::Mutex;

use crate::{
    auth::users::User,
    contests::{Contest, Participant},
    db::DbPoolConnection,
};

use super::scoring::ParticipantScores;

pub struct Leaderboard {
    pub contest: Contest,
    pub scores: Vec<ParticipantScores>,
}

#[derive(Serialize)]
pub struct LeaderboardEntry {
    pub user: User,
    pub problems_completed: Vec<i64>,
}

impl Leaderboard {
    pub async fn new(db: &mut DbPoolConnection, contest: Contest) -> Self {
        let scores = Self::get_scores(db, &contest).await;
        Self { contest, scores }
    }

    async fn get_scores(db: &mut DbPoolConnection, contest: &Contest) -> Vec<ParticipantScores> {
        let participants = Participant::list_not_judge(db, contest.id).await;
        let mut scores = Vec::new();
        for p in participants {
            scores.push(ParticipantScores::new(db, &p, contest).await);
        }
        scores.sort();
        scores
    }

    // pub async fn standings(&self) -> Vec<(i64, Vec<i64>)> {
    //     self.scores
    //         .iter()
    //         .map(|s| {
    //             (
    //                 s.participant_id,
    //                 s.scores.iter().map(|(id, _)| *id).collect(),
    //             )
    //         })
    //         .collect()
    // }

    pub async fn full(&self, db: &mut DbPoolConnection) -> Vec<LeaderboardEntry> {
        let cases = self
            .scores
            .iter()
            .enumerate()
            .map(|(i, s)| format!("WHEN {} THEN {}", s.participant_id, i))
            .collect::<Vec<_>>()
            .join(" ");
        let scores = self
            .scores
            .iter()
            .map(|s| {
                (
                    s.user_id,
                    s.scores.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();
        let query = format!(
            "
            SELECT user.* FROM participant 
            JOIN user ON participant.user_id = user.id 
            WHERE contest_id = ? AND is_judge = false
            ORDER BY CASE participant.p_id {} ELSE 0 END DESC;
        ",
            if cases.is_empty() {
                "WHEN 0 THEN 0"
            } else {
                &cases
            }
        );
        let res = sqlx::query(query.trim())
            .bind(self.contest.id)
            .fetch_all(&mut **db)
            .await
            .unwrap();
        res.into_iter()
            .map(|row| {
                let user = User::from_row(&row).unwrap();
                let problems_completed = scores.get(&user.id).unwrap();
                LeaderboardEntry {
                    user,
                    problems_completed: problems_completed.clone(),
                }
            })
            .collect::<Vec<_>>()
    }

    pub async fn update(&mut self, db: &mut DbPoolConnection, updated_participant: i64) {
        let participant = self
            .scores
            .iter_mut()
            .find(|s| s.participant_id == updated_participant);
        if let Some(p) = participant {
            p.refresh(db).await;
            self.scores.sort();
        }
    }

    pub async fn full_refresh(&mut self, db: &mut DbPoolConnection, contest: Option<&Contest>) {
        if let Some(c) = contest {
            self.contest = c.clone();
            for s in &mut self.scores {
                s.update_contest(db, c).await;
            }
        }
        self.scores = Self::get_scores(db, &self.contest).await;
    }
}

pub struct LeaderboardManager {
    leaderboards: HashMap<i64, Arc<Mutex<Leaderboard>>>,
}

impl LeaderboardManager {
    pub async fn new() -> Self {
        Self {
            leaderboards: HashMap::new(),
        }
    }

    pub async fn get_leaderboard(
        &mut self,
        db: &mut DbPoolConnection,
        contest: &Contest,
    ) -> Arc<Mutex<Leaderboard>> {
        if let Some(leaderboard) = self.leaderboards.get(&contest.id) {
            leaderboard.clone()
        } else {
            let leaderboard = Leaderboard::new(db, contest.clone()).await;
            let leaderboard = Arc::new(Mutex::new(leaderboard));
            self.leaderboards.insert(contest.id, leaderboard.clone());
            leaderboard
        }
    }
}

pub type LeaderboardManagerHandle = Arc<Mutex<LeaderboardManager>>;
