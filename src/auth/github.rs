use log::error;
use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar, SameSite, Status},
    response::Redirect,
    routes,
};
use rocket_oauth2::{OAuth2, TokenResponse};

use crate::db::{DbConnection, DbPoolConnection};

use super::{users::User, CallbackHandler};

pub struct GitHubLogin(pub String);

const SCOPES: [&str; 1] = ["user:read"];

#[derive(Debug, serde::Deserialize)]
pub struct UserInfo {
    pub id: i64,
}

#[get("/login")]
fn github_login(oauth2: OAuth2<GitHubLogin>, cookies: &CookieJar<'_>) -> Result<Redirect, Status> {
    let mut cookie = Cookie::new("state-oauth-type", "login");
    cookie.set_same_site(SameSite::None);
    cookies.add(cookie);
    oauth2.get_redirect(cookies, &SCOPES).map_err(|e| {
        error!("Error getting GitHub redirect: {}", e);
        Status::InternalServerError
    })
}

#[get("/link")]
fn github_link(
    oauth2: OAuth2<GitHubLogin>,
    _user: &User,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let mut cookie = Cookie::new("state-oauth-type", "link");
    cookie.set_same_site(SameSite::None);
    cookies.add(cookie);
    oauth2.get_redirect(cookies, &SCOPES).map_err(|e| {
        error!("Error getting GitHub redirect: {}", e);
        Status::InternalServerError
    })
}

#[get("/callback")]
async fn github_callback(
    mut db: DbConnection,
    token: TokenResponse<GitHubLogin>,
    user: Option<&User>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let handler = GitHubLogin(token.access_token().to_string());

    let state = cookies
        .get("state-oauth-type")
        .map(|c| c.value())
        .ok_or(Status::BadRequest)?;

    let res = if state == "login" {
        handler.handle_callback(&mut db, cookies).await
    } else if state == "link" && user.is_some() {
        handler.handle_link_callback(&mut db, user.unwrap()).await
    } else {
        return Err(Status::BadRequest);
    }
    .map_err(|e| {
        error!("Error handling GitHub callback: {}", e);
        Status::InternalServerError
    })?;

    cookies.remove(Cookie::from("state-oauth-type"));

    res
}

#[get("/unlink")]
async fn github_unlink(mut db: DbConnection, user: &User) -> Result<Redirect, Status> {
    let res = sqlx::query!("UPDATE user SET github_id = NULL WHERE id = ?", user.id)
        .execute(&mut **db)
        .await
        .map_err(|e| {
            error!("Error unlinking GitHub account: {}", e);
            Status::InternalServerError
        })?;
    if res.rows_affected() == 1 {
        Ok(Redirect::to("/settings/account"))
    } else {
        Err(Status::InternalServerError)
    }
}

#[rocket::async_trait]
impl CallbackHandler for GitHubLogin {
    type IntermediateUserInfo = UserInfo;

    const SERVICE_NAME: &'static str = "GitHub";

    fn get_request_client(&self) -> reqwest::RequestBuilder {
        reqwest::Client::new()
            .get("https://api.github.com/user")
            .header("User-Agent", "Test-App")
            .header(
                "Accept",
                "application/vnd.github+json,application/vnd.github.diff",
            )
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {}", self.0))
    }

    async fn get_user(
        &self,
        db: &mut DbPoolConnection,
        user: Self::IntermediateUserInfo,
    ) -> Result<Option<User>, String> {
        sqlx::query_as!(User, "SELECT * FROM user WHERE github_id = ?", user.id)
            .fetch_optional(&mut **db)
            .await
            .map_err(|e| e.to_string())
    }

    async fn link_to(
        &self,
        db: &mut DbPoolConnection,
        user: &User,
        user_info: Self::IntermediateUserInfo,
    ) -> Result<bool, String> {
        sqlx::query!(
            "UPDATE user SET github_id = ? WHERE id = ?",
            user_info.id,
            user.id
        )
        .execute(&mut **db)
        .await
        .map(|r| r.rows_affected() == 1)
        .map_err(|e| e.to_string())
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("GitHub Auth", |rocket| async {
        rocket
            .attach(OAuth2::<GitHubLogin>::fairing("github"))
            .mount(
                "/auth/github",
                routes![github_login, github_callback, github_link, github_unlink,],
            )
    })
}
