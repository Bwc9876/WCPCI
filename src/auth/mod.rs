#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use log::error;
use rocket::{
    catch, catchers, fairing::AdHoc, form::Form, get, http::CookieJar, post, response::Redirect,
    routes, FromForm,
};
use rocket_dyn_templates::Template;
use rocket_oauth2::{OAuth2, TokenResponse};

use crate::{
    context_with_base,
    db::{DbConnection, DbPool},
};

use self::{github::GitHubLogin, google::GoogleLogin, sessions::Session, users::User};

mod github;
mod google;

pub mod sessions;
pub mod users;

#[catch(401)]
async fn unauthorized() -> Redirect {
    Redirect::to("/auth/login")
}

#[get("/login")]
async fn login(user: Option<&User>) -> Template {
    let ctx = context_with_base!(user,);
    Template::render("auth/login", ctx)
}

#[get("/logout")]
async fn logout(mut db: DbConnection, cookies: &CookieJar<'_>) -> Redirect {
    if let Some(token) = cookies
        .get_private(Session::TOKEN_COOKIE_NAME)
        .map(|c| c.value().to_string())
    {
        if let Some(session) = Session::from_token(&mut db, &token).await {
            let res = sqlx::query!("DELETE FROM session WHERE id = ?", session.id)
                .execute(&mut **db)
                .await;
            if let Err(why) = res {
                error!("Failed to delete session {}: {:?}", session.id, why);
            }
        }
        cookies.remove_private(Session::TOKEN_COOKIE_NAME);
    }
    Redirect::to("/")
}

#[get("/login/github")]
fn github_login(oauth2: OAuth2<GitHubLogin>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["user:read"]).unwrap()
}

#[get("/callback/github")]
async fn github_callback(
    mut db: DbConnection,
    token: TokenResponse<GitHubLogin>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    //println!("State cookie is: {:?}", cookies.get_private("rocket_oauth2_state"));
    let handler = GitHubLogin(token.access_token().to_string());
    handler.handle_callback(&mut db, cookies).await
    //Redirect::to("/")
}

#[get("/login/google")]
fn google_login(oauth2: OAuth2<GoogleLogin>, cookies: &CookieJar<'_>) -> Redirect {
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
async fn google_callback(
    mut db: DbConnection,
    token: TokenResponse<GoogleLogin>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    let handler = GoogleLogin(token.access_token().to_string());
    handler.handle_callback(&mut db, cookies).await
}

#[derive(FromForm)]
struct DebugLoginForm<'r> {
    email: &'r str,
}

#[post("/login/debug", data = "<debug_login_form>")]
async fn debug_login(
    mut db: DbConnection,
    debug_login_form: Form<DebugLoginForm<'_>>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    let email = debug_login_form.email;
    let user = User::temporary(email.to_string(), email.to_string());
    if let Err(why) = User::login_oauth(&mut db, cookies, user).await {
        error!("Failed to log in user: {:?}", why);
    }
    Redirect::to("/")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Auth App", |rocket| async {
        let rocket = rocket
            .attach(OAuth2::<GitHubLogin>::fairing("github"))
            .attach(OAuth2::<GoogleLogin>::fairing("google"))
            .register("/", catchers![unauthorized])
            .mount(
                "/auth",
                routes![
                    login,
                    logout,
                    github_login,
                    github_callback,
                    google_login,
                    google_callback
                ],
            );

        if cfg!(debug_assertions) {
            rocket.mount("/auth", routes![debug_login])
        } else {
            rocket
        }
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
