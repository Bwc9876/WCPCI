use chrono::NaiveDateTime;
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    outcome::IntoOutcome,
    request::{self, FromRequest},
    time::OffsetDateTime,
    FromFormField, Request, State,
};
use serde::Serialize;
use sqlx::{encode::IsNull, prelude::FromRow, Decode, Encode, Type};

use crate::db::{DbConnection, DbPoolConnection};

use super::sessions::Session;

#[derive(Debug, Clone, Serialize, FromFormField)]
pub enum ColorScheme {
    Light,
    Dark,
    UseSystem,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::UseSystem
    }
}

impl From<String> for ColorScheme {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Light" => Self::Light,
            "Dark" => Self::Dark,
            "UseSystem" => Self::UseSystem,
            _ => Self::UseSystem,
        }
    }
}

impl From<ColorScheme> for String {
    fn from(s: ColorScheme) -> Self {
        format!("{:?}", s)
    }
}

impl Type<sqlx::Sqlite> for ColorScheme {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as Type<sqlx::Sqlite>>::type_info()
    }
}

impl Encode<'_, sqlx::Sqlite> for ColorScheme {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::database::HasArguments<'_>>::ArgumentBuffer,
    ) -> IsNull {
        let val = format!("{:?}", self);
        <std::string::String as Encode<'_, sqlx::Sqlite>>::encode_by_ref(&val, buf)
    }
}

impl Decode<'_, sqlx::Sqlite> for ColorScheme {
    fn decode(
        value: <sqlx::Sqlite as sqlx::database::HasValueRef<'_>>::ValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <String as Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(s.into())
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub bio: String,
    pub default_display_name: String,
    pub display_name: Option<String>,
    pub color_scheme: ColorScheme,
    pub default_language: String,
    pub created_at: NaiveDateTime,
}

pub trait UserMigration {
    fn migrate(self, default_language: &str) -> User;
}

impl User {
    pub fn display_name(&self) -> &str {
        self.display_name
            .as_ref()
            .unwrap_or(&self.default_display_name)
    }

    pub fn temporary(email: String, display_name: String, default_language: &str) -> Self {
        Self {
            id: 0,
            bio: String::new(),
            email,
            default_display_name: display_name,
            color_scheme: ColorScheme::default(),
            default_language: default_language.to_string(),
            display_name: None,
            created_at: chrono::offset::Utc::now().naive_utc(),
        }
    }

    pub async fn login_oauth<'a>(
        db: &mut DbPoolConnection,
        cookies: &'a CookieJar<'a>,
        data: impl UserMigration,
        default_language: &str,
    ) -> Result<(), String> {
        let user: User = data.migrate(default_language);

        let existing = Self::get_by_email(db, &user.email).await.unwrap();

        let user = if let Some(user) = existing {
            user
        } else {
            user.insert(db).await.map_err(|e| e.to_string())?
        };

        let session = Session::create(db, user.id).await?;

        let expires =
            OffsetDateTime::from_unix_timestamp(session.expires_at.and_utc().timestamp()).unwrap();

        cookies.add_private(
            Cookie::build(("token", session.token))
                .same_site(SameSite::Lax)
                .expires(expires)
                .build(),
        );

        Ok(())
    }

    pub async fn insert(self, db: &mut DbPoolConnection) -> Result<User, String> {
        let new = sqlx::query_as!(
            User,
            "INSERT INTO user (email, default_display_name, color_scheme, default_language) VALUES (?, ?, ?, ?) RETURNING *",
            self.email,
            self.default_display_name,
            self.color_scheme,
            self.default_language
        )
        .fetch_one(&mut **db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(new)
    }

    pub async fn delete(&self, db: &mut DbPoolConnection) -> Result<(), String> {
        let res = sqlx::query!("DELETE FROM user WHERE id = ?", self.id)
            .execute(&mut **db)
            .await;

        res.map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn get(db: &mut DbPoolConnection, id: i64) -> Option<User> {
        sqlx::query_as!(User, "SELECT * FROM user WHERE id = ?", id)
            .fetch_optional(&mut **db)
            .await
            .ok()
            .flatten()
    }

    pub async fn list(db: &mut DbPoolConnection) -> Result<Vec<User>, String> {
        let users: Vec<User> = sqlx::query_as!(User, "SELECT * FROM user")
            .fetch_all(&mut **db)
            .await
            .map_err(|e| e.to_string())?;

        Ok(users)
    }

    async fn get_by_email(
        db: &mut DbPoolConnection,
        username: &str,
    ) -> Result<Option<User>, String> {
        let user: Option<User> =
            sqlx::query_as!(User, "SELECT * FROM user WHERE email = ?", username)
                .fetch_optional(&mut **db)
                .await
                .map_err(|e| e.to_string())?;

        Ok(user)
    }
}

pub struct AdminUsers(pub Vec<String>);

pub struct Admin();

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

        user_result.as_ref().or_error((Status::Unauthorized, ()))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Admin {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let admin_result = req
            .local_cache_async(async {
                let user = req.guard::<&User>().await.succeeded()?;
                let admin_users = req.guard::<&State<AdminUsers>>().await.succeeded()?;
                if admin_users.0.contains(&user.email) {
                    Some(Admin())
                } else {
                    None
                }
            })
            .await;
        admin_result.as_ref().or_error((Status::Forbidden, ()))
    }
}
