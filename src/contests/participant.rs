use chrono::NaiveDateTime;

use crate::{auth::users::User, db::DbPoolConnection};

#[derive(Serialize)]
pub struct Participant {
    pub p_id: i64,
    pub user_id: i64,
    contest_id: i64,
    pub is_judge: bool,
    registered_at: Option<NaiveDateTime>,
}

impl Participant {
    pub async fn get(db: &mut DbPoolConnection, contest_id: i64, user_id: i64) -> Option<Self> {
        sqlx::query_as!(
            Participant,
            "SELECT * FROM participant WHERE contest_id = ? AND user_id = ?",
            contest_id,
            user_id
        )
        .fetch_optional(&mut **db)
        .await
        .ok()
        .flatten()
    }

    // pub async fn by_id(db: &mut DbPoolConnection, p_id: i64) -> Option<Self> {
    //     sqlx::query_as!(
    //         Participant,
    //         "SELECT * FROM participant WHERE p_id = ?",
    //         p_id
    //     )
    //     .fetch_optional(&mut **db)
    //     .await
    //     .ok()
    //     .flatten()
    // }

    pub async fn list(db: &mut DbPoolConnection, contest_id: i64) -> Vec<(Self, User)> {
        let res = sqlx::query!("SELECT participant.*, user.* FROM participant JOIN user ON participant.user_id = user.id WHERE contest_id = ?", contest_id)
            .fetch_all(&mut **db)
            .await
            .unwrap();
        res.into_iter()
            .map(|row| {
                let participant = Participant {
                    p_id: row.p_id,
                    user_id: row.user_id,
                    contest_id: row.contest_id,
                    is_judge: row.is_judge,
                    registered_at: row.registered_at,
                };
                let user = User {
                    id: row.id,
                    bio: row.bio,
                    sso_id: row.sso_id,
                    profile_picture_source: row.profile_picture_source,
                    color_scheme: row.color_scheme.into(),
                    default_language: row.default_language,
                    display_name: row.display_name,
                    default_display_name: row.default_display_name,
                    email: row.email,
                    created_at: row.created_at,
                    github_id: row.github_id,
                    google_id: row.google_id,
                    gravatar_email: row.gravatar_email,
                };
                (participant, user)
            })
            .collect()
    }

    pub async fn list_judge(db: &mut DbPoolConnection, contest_id: i64) -> Vec<User> {
        sqlx::query_as!(
            User,
            "SELECT user.* FROM participant JOIN user ON participant.user_id = user.id WHERE contest_id = ? AND is_judge = true",
            contest_id
        )
        .fetch_all(&mut **db)
        .await
        .unwrap()
    }

    pub async fn list_not_judge(db: &mut DbPoolConnection, contest_id: i64) -> Vec<Self> {
        sqlx::query_as!(
            Participant,
            "SELECT * FROM participant WHERE contest_id = ? AND is_judge = false",
            contest_id
        )
        .fetch_all(&mut **db)
        .await
        .unwrap()
    }

    pub async fn insert(&self, db: &mut DbPoolConnection) -> Result<Participant, sqlx::Error> {
        sqlx::query_as!(
            Participant,
            "INSERT INTO participant (user_id, contest_id, is_judge, registered_at) VALUES (?, ?, ?, ?) RETURNING *",
            self.user_id,
            self.contest_id,
            self.is_judge,
            self.registered_at
        )
        .fetch_one(&mut **db)
        .await
    }

    pub async fn remove(
        db: &mut DbPoolConnection,
        contest_id: i64,
        user_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM participant WHERE contest_id = ? AND user_id = ?",
            contest_id,
            user_id
        )
        .execute(&mut **db)
        .await
        .map(|_| ())
    }

    pub async fn create_or_make_judge(
        db: &mut DbPoolConnection,
        contest_id: i64,
        user_id: i64,
    ) -> Result<Participant, sqlx::Error> {
        sqlx::query_as!(
            Participant,
            "INSERT INTO participant (user_id, contest_id, is_judge) VALUES (?, ?, true) ON CONFLICT (user_id, contest_id) DO UPDATE SET is_judge = true RETURNING *",
            user_id,
            contest_id
        ).fetch_one(&mut **db).await
    }

    // pub async fn update(&self, db: &mut DbPoolConnection) -> Result<(), sqlx::Error> {
    //     sqlx::query_as!(
    //         Participant,
    //         "UPDATE participant SET is_judge = ? WHERE user_id = ? AND contest_id = ?",
    //         self.is_judge,
    //         self.user_id,
    //         self.contest_id
    //     )
    //     .execute(&mut **db)
    //     .await.map(|_| ())
    // }

    pub fn temp(user_id: i64, contest_id: i64, is_judge: bool) -> Self {
        Self {
            p_id: 0,
            user_id,
            contest_id,
            is_judge,
            registered_at: None,
        }
    }
}
