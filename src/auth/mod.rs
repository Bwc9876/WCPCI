use log::error;
use rocket::{catch, catchers, fairing::AdHoc, get, http::CookieJar, response::Redirect, routes};
use rocket_oauth2::{OAuth2, TokenResponse};

use crate::db::{DbConnection, DbPool};

use self::{github::GitHubLogin, google::GoogleLogin, users::User};

mod github;
mod google;

pub mod sessions;
pub mod users;

#[catch(401)]
async fn unauthorized() -> Redirect {
    Redirect::to("/auth/login")
}

#[get("/login")]
async fn login() -> &'static str {
    "Login pls lol"
}

#[get("/login/github")]
pub fn github_login(oauth2: OAuth2<GitHubLogin>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["user:read"]).unwrap()
}

#[get("/callback/github")]
pub async fn github_callback(
    mut db: DbConnection,
    token: TokenResponse<GitHubLogin>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    let handler = GitHubLogin(token.access_token().to_string());
    handler.handle_callback(&mut db, cookies).await
}

#[get("/login/google")]
pub fn google_login(oauth2: OAuth2<GoogleLogin>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2
        .get_redirect(
            cookies,
            &[
                "https://www.googleapis.com/auth/userinfo.email",
                "https://www.googleapis.com/auth/userinfo.profile",
            ],
        )
        .unwrap()
}

#[get("/callback/google")]
pub async fn google_callback(
    mut db: DbConnection,
    token: TokenResponse<GoogleLogin>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    let handler = GoogleLogin(token.access_token().to_string());
    handler.handle_callback(&mut db, cookies).await
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Auth App", |rocket| async {
        rocket
            .attach(OAuth2::<GitHubLogin>::fairing("github"))
            .attach(OAuth2::<GoogleLogin>::fairing("google"))
            .register("/", catchers![unauthorized])
            .mount(
                "/auth",
                routes![
                    login,
                    github_login,
                    github_callback,
                    google_login,
                    google_callback
                ],
            )
    })
}

#[rocket::async_trait]
trait CallbackHandler {
    type IntermediateUserInfo: serde::de::DeserializeOwned + Into<User> + Sync + Send;
    const SERVICE_NAME: &'static str;

    fn get_request_client(&self) -> reqwest::RequestBuilder;

    async fn handle_callback(
        &self,
        db: &mut DbPool,
        cookies: &CookieJar<'_>,
    ) -> rocket::response::Redirect {
        match self.get_request_client().send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<Self::IntermediateUserInfo>().await {
                        Ok(user_info) => {
                            let db_conn = &mut *db;
                            if let Err(why) = User::login_oauth(db_conn, cookies, user_info).await {
                                error!("Failed to log in user: {:?}", why);
                            }
                        }
                        Err(why) => {
                            error!(
                                "Failed to parse user info from {}: {why:?}",
                                Self::SERVICE_NAME
                            );
                        }
                    }
                } else {
                    error!(
                        "Failed to get user info from {}: {:?}",
                        Self::SERVICE_NAME,
                        resp.text().await
                    );
                }
            }
            Err(why) => {
                error!(
                    "Failed to send request to {}: {:?}",
                    Self::SERVICE_NAME,
                    why
                );
            }
        }

        Redirect::to("/")
    }
}
