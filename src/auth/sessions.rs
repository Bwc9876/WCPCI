use chrono::NaiveDateTime;
use rand::{rngs::OsRng, RngCore};

use crate::{db::DbPoolConnection, error::prelude::*};

pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

impl Session {
    pub const TOKEN_COOKIE_NAME: &'static str = "token";
    const TOKEN_LENGTH: usize = 64;
    const EXPIRY_DAYS: i64 = 14;

    fn gen_token() -> String {
        let mut token = Vec::with_capacity(Self::TOKEN_LENGTH);

        let mut rng = OsRng;

        for _ in 0..Self::TOKEN_LENGTH {
            let random_byte = (rng.next_u32() % 256) as u8;
            token.push(format!("{:02x}", random_byte));
        }

        token.join("")
    }

    pub async fn create(db: &mut DbPoolConnection, user_id: i64) -> Result<Session> {
        let token = Self::gen_token();
        let now = chrono::offset::Utc::now();
        let expires = now
            + chrono::TimeDelta::try_days(Self::EXPIRY_DAYS)
                .context("Failed to set expiry days")?;

        let session = sqlx::query_as!(Session, "INSERT INTO session (user_id, token, created_at, expires_at) VALUES (?, ?, ?, ?) RETURNING *", user_id, token, now, expires)
            .fetch_one(&mut **db).await.context("Couldn't insert new session")?;

        Ok(session)
    }

    pub async fn from_token(db: &mut DbPoolConnection, token: &str) -> Result<Option<Session>> {
        let ses = sqlx::query_as!(
            Session,
            "SELECT * FROM session WHERE session.token = ? AND expires_at > CURRENT_TIMESTAMP",
            token
        )
        .fetch_optional(&mut **db)
        .await?;
        Ok(ses)
    }
}
