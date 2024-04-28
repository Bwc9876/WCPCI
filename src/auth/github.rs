use rocket::{
    fairing::AdHoc,
    get,
    http::{Cookie, CookieJar, SameSite, Status},
    response::Redirect,
    routes,
};
use rocket_oauth2::{OAuth2, TokenResponse};

use crate::{
    db::{DbConnection, DbPoolConnection},
    error::prelude::*,
    messages::Message,
};

use super::{users::User, CallbackHandler};

pub struct GitHubLogin(pub String);

const SCOPES: [&str; 1] = ["user:read"];

#[derive(Debug, serde::Deserialize)]
pub struct UserInfo {
    pub id: i64,
}

#[get("/login")]
fn github_login(oauth2: OAuth2<GitHubLogin>, cookies: &CookieJar<'_>) -> ResultResponse<Redirect> {
    let mut cookie = Cookie::new("state-oauth-type", "login");
    cookie.set_secure(false);
    cookie.set_same_site(SameSite::Lax);
    cookies.add(cookie);
    let redirect = oauth2
        .get_redirect(cookies, &SCOPES)
        .context("Error getting GitHub redirect")?;
    Ok(redirect)
}

#[get("/link")]
fn github_link(
    oauth2: OAuth2<GitHubLogin>,
    _user: &User,
    cookies: &CookieJar<'_>,
) -> ResultResponse<Redirect> {
    let mut cookie = Cookie::new("state-oauth-type", "link");
    cookie.set_secure(false);
    cookie.set_same_site(SameSite::Lax);
    cookies.add(cookie);
    let redirect = oauth2
        .get_redirect(cookies, &SCOPES)
        .context("Error getting GitHub redirect")?;
    Ok(redirect)
}

#[get("/callback")]
async fn github_callback(
    mut db: DbConnection,
    token: TokenResponse<GitHubLogin>,
    user: Option<&User>,
    cookies: &CookieJar<'_>,
) -> ResultResponse<Redirect> {
    let handler = GitHubLogin(token.access_token().to_string());

    let state = cookies
        .get("state-oauth-type")
        .map(|c| c.value())
        .ok_or_else(|| {
            error!("No state-type cookie found for GitHub callback");
            Status::BadRequest
        })?;

    let res = if state == "login" {
        handler.handle_callback(&mut db, cookies).await
    } else if state == "link" && user.is_some() {
        handler.handle_link_callback(&mut db, user.unwrap()).await
    } else {
        return Err(Status::BadRequest.into());
    }
    .context("Error handling GitHub callback")??;

    cookies.remove(Cookie::from("state-oauth-type"));

    Ok(res)
}

#[get("/unlink")]
async fn github_unlink(mut db: DbConnection, user: &User) -> ResultResponse<Redirect> {
    let res = sqlx::query!("UPDATE user SET github_id = NULL WHERE id = ?", user.id)
        .execute(&mut **db)
        .await
        .context("Error unlinking GitHub account: {}")?;
    if res.rows_affected() == 1 {
        Ok(Message::success("GitHub account unlinked").to("/settings/account"))
    } else {
        Err(Status::InternalServerError.into())
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
    ) -> Result<Option<User>> {
        let res = sqlx::query_as!(User, "SELECT * FROM user WHERE github_id = ?", user.id)
            .fetch_optional(&mut **db)
            .await?;
        Ok(res)
    }

    async fn link_to(
        &self,
        db: &mut DbPoolConnection,
        user: &User,
        user_info: Self::IntermediateUserInfo,
    ) -> Result<bool> {
        let other_exists = sqlx::query!(
            "SELECT * FROM user WHERE github_id = ? AND id != ?",
            user_info.id,
            user.id
        )
        .fetch_optional(&mut **db)
        .await?
        .is_some();

        if other_exists {
            return Ok(false);
        }

        let res = sqlx::query!(
            "UPDATE user SET github_id = ? WHERE id = ?",
            user_info.id,
            user.id
        )
        .execute(&mut **db)
        .await
        .map(|r| r.rows_affected() == 1)?;
        Ok(res)
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
