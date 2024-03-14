use anyhow::Result;
use rand::{rngs::OsRng, RngCore};
use rocket::time::OffsetDateTime;

use crate::db::DbPool;

pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
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

    pub async fn create(db: &mut DbPool, user_id: i64) -> Result<Session> {
        let token = Self::gen_token();
        let now = OffsetDateTime::now_utc();
        let expires = OffsetDateTime::from_unix_timestamp(
            now.unix_timestamp() + 60 * 60 * 24 * Self::EXPIRY_DAYS,
        )?;

        let session = sqlx::query_as!(Session, "INSERT INTO session (user_id, token, created_at, expires_at) VALUES (?, ?, ?, ?) RETURNING *", user_id, token, now, expires)
            .fetch_one(&mut **db).await?;

        Ok(session)
    }

    pub async fn from_token(db: &mut DbPool, token: &str) -> Option<Session> {
        sqlx::query_as!(
            Session,
            "SELECT * FROM session WHERE session.token = ? AND expires_at > CURRENT_TIMESTAMP",
            token
        )
        .fetch_one(&mut **db)
        .await
        .ok()
    }
}
