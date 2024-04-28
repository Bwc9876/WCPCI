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
};

use super::{users::User, CallbackHandler};

pub struct GoogleLogin(pub String);

const SCOPES: [&str; 2] = [
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

#[derive(Debug, serde::Deserialize)]
pub struct UserInfo {
    pub user_id: String,
}

#[get("/login")]
fn google_login(oauth2: OAuth2<GoogleLogin>, cookies: &CookieJar<'_>) -> ResultResponse<Redirect> {
    let mut cookie = Cookie::new("state-oauth-type", "login");
    cookie.set_same_site(SameSite::None);
    cookies.add(cookie);
    let redirect = oauth2
        .get_redirect(cookies, &SCOPES)
        .context("Error getting Google redirect")?;
    Ok(redirect)
}

#[get("/link")]
fn google_link(
    oauth2: OAuth2<GoogleLogin>,
    _user: &User,
    cookies: &CookieJar<'_>,
) -> ResultResponse<Redirect> {
    let mut cookie = Cookie::new("state-oauth-type", "link");
    cookie.set_same_site(SameSite::None);
    cookies.add(cookie);
    let redirect = oauth2
        .get_redirect(cookies, &SCOPES)
        .context("Error getting Google redirect")?;
    Ok(redirect)
}

#[get("/callback")]
async fn google_callback(
    mut db: DbConnection,
    token: TokenResponse<GoogleLogin>,
    user: Option<&User>,
    cookies: &CookieJar<'_>,
) -> ResultResponse<Redirect> {
    let handler = GoogleLogin(token.access_token().to_string());
    let state = cookies
        .get("state-oauth-type")
        .map(|c| c.value())
        .ok_or(Status::BadRequest)?;

    let res = if state == "login" {
        handler.handle_callback(&mut db, cookies).await
    } else if state == "link" && user.is_some() {
        handler.handle_link_callback(&mut db, user.unwrap()).await
    } else {
        return Err(Status::BadRequest.into());
    }
    .context("Error handling Google callback")??;

    cookies.remove(Cookie::from("state-oauth-type"));

    Ok(res)
}

#[get("/unlink")]
async fn google_unlink(mut db: DbConnection, user: &User) -> ResultResponse<Redirect> {
    let res = sqlx::query!("UPDATE user SET google_id = NULL WHERE id = ?", user.id)
        .execute(&mut **db)
        .await
        .context("Error unlinking Google account")?;
    if res.rows_affected() == 1 {
        Ok(Redirect::to("/settings/account"))
    } else {
        Err(Status::InternalServerError.into())
    }
}

#[rocket::async_trait]
impl CallbackHandler for GoogleLogin {
    type IntermediateUserInfo = UserInfo;

    const SERVICE_NAME: &'static str = "Google";

    fn get_request_client(&self) -> reqwest::RequestBuilder {
        reqwest::Client::new()
            .get(format!(
                "https://www.googleapis.com/oauth2/v1/tokeninfo?access_token={}",
                self.0
            ))
            .header("User-Agent", "Test-App")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.0))
    }

    async fn get_user(
        &self,
        db: &mut DbPoolConnection,
        user: Self::IntermediateUserInfo,
    ) -> Result<Option<User>> {
        let res = sqlx::query_as!(User, "SELECT * FROM user WHERE google_id = ?", user.user_id)
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
            "SELECT * FROM user WHERE google_id = ? AND id != ?",
            user_info.user_id,
            user.id
        )
        .fetch_optional(&mut **db)
        .await?
        .is_some();

        if other_exists {
            return Ok(false);
        }

        let res = sqlx::query!(
            "UPDATE user SET google_id = ? WHERE id = ?",
            user_info.user_id,
            user.id
        )
        .execute(&mut **db)
        .await
        .map(|r| r.rows_affected() == 1)?;
        Ok(res)
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Google Auth", |rocket| async {
        rocket
            .attach(OAuth2::<GoogleLogin>::fairing("google"))
            .mount(
                "/auth/google",
                routes![google_login, google_callback, google_link, google_unlink,],
            )
    })
}
