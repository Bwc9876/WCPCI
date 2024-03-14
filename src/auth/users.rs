use anyhow::Result;
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    outcome::IntoOutcome,
    request::{self, FromRequest},
    time::OffsetDateTime,
    Request,
};
use serde::Serialize;

use crate::db::{DbConnection, DbPool};

use super::sessions::Session;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub bio: String,
    pub default_display_name: String,
    pub display_name: Option<String>,
    #[serde(skip)] // Not implemented, I cry
    pub created_at: OffsetDateTime,
}

impl User {
    pub fn display_name(&self) -> &str {
        self.display_name
            .as_ref()
            .unwrap_or(&self.default_display_name)
    }

    pub fn temporary(email: String, display_name: String) -> Self {
        Self {
            id: 0,
            bio: String::new(),
            email,
            default_display_name: display_name,
            display_name: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub async fn login_oauth<'a>(
        db: &mut DbPool,
        cookies: &'a CookieJar<'a>,
        data: impl Into<User>,
    ) -> Result<()> {
        let user: User = data.into();

        let existing = Self::get_by_email(db, &user.email).await.unwrap();

        let user = if let Some(user) = existing {
            user
        } else {
            user.write_to_db(db).await?
        };

        let session = Session::create(db, user.id).await?;

        cookies.add_private(
            Cookie::build(("token", session.token))
                .same_site(SameSite::Lax)
                .expires(session.expires_at)
                .build(),
        );

        Ok(())
    }

    pub async fn write_to_db(self, db: &mut DbPool) -> Result<User> {
        let new = sqlx::query_as!(
            User,
            "INSERT INTO user (email, default_display_name) VALUES (?, ?) RETURNING *",
            self.email,
            self.default_display_name
        )
        .fetch_one(&mut **db)
        .await?;

        Ok(new)
    }

    async fn get_by_email(db: &mut DbPool, username: &str) -> Result<Option<User>> {
        let user: Option<User> =
            sqlx::query_as!(User, "SELECT * FROM user WHERE email = ?", username)
                .fetch_optional(&mut **db)
                .await?;

        Ok(user)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r User {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user_result = req.local_cache_async(async {
            let mut db = req.guard::<DbConnection>().await.succeeded()?;
            if let Some(token) = req.cookies().get_private(Session::TOKEN_COOKIE_NAME).map(|c| c.value().to_string()) {
                sqlx::query_as!(
                    User,
                    "SELECT user.* FROM user JOIN session ON user.id = session.user_id WHERE session.token = ? AND expires_at > CURRENT_TIMESTAMP",
                    token
                )
                .fetch_one(&mut **db)
                .await.ok()
            } else {
                None
            }
        }).await;

        user_result.as_ref().or_forward(Status::Unauthorized)
    }
}
