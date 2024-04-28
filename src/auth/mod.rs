#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use log::warn;
use rocket::{
    catch, catchers,
    fairing::AdHoc,
    get,
    http::{CookieJar, Status},
    response::Redirect,
    routes,
};
use rocket_dyn_templates::Template;

use crate::{
    context_with_base,
    db::{DbConnection, DbPoolConnection},
    error::prelude::*,
    messages::Message,
    ResultResponse,
};

use self::{
    sessions::Session,
    users::{AdminUsers, User},
};

mod github;
mod google;
mod saml;

pub use saml::{SamlOptions, PREFERRED_SSO_BINDING};

pub mod csrf;
pub mod sessions;
pub mod users;

#[catch(401)]
async fn unauthorized() -> Redirect {
    Message::info("You need to be logged in to access this page").to("/auth/login")
}

#[get("/login")]
async fn login(user: Option<&User>) -> Template {
    let ctx = context_with_base!(user,);
    Template::render("auth/login", ctx)
}

#[get("/logout")]
async fn logout(mut db: DbConnection, cookies: &CookieJar<'_>) -> ResultResponse<Redirect> {
    if let Some(token) = cookies
        .get_private(Session::TOKEN_COOKIE_NAME)
        .map(|c| c.value().to_string())
    {
        let session = Session::from_token(&mut db, &token)
            .await
            .with_context(|| format!("Couldn't get session with token: {token}"))?;
        if let Some(session) = session {
            sqlx::query!("DELETE FROM session WHERE id = ?", session.id)
                .execute(&mut **db)
                .await
                .map_err(|why| anyhow!("Failed to delete session {}: {why:?}", session.id))?;
        }

        cookies.remove_private(Session::TOKEN_COOKIE_NAME);
    }
    Ok(Message::success("Logged out").to("/"))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Auth App", |rocket| async {
        let admins: Vec<String> = rocket
            .figment()
            .extract_inner("admins")
            .unwrap_or_else(|_| {
                warn!("No admin user specified");
                Vec::new()
            });
        rocket
            .manage(AdminUsers(admins))
            .attach(saml::stage())
            .attach(github::stage())
            .attach(google::stage())
            .attach(csrf::stage())
            .register("/", catchers![unauthorized])
            .mount("/auth", routes![login, logout,])
    })
}

#[rocket::async_trait]
trait CallbackHandler {
    type IntermediateUserInfo: serde::de::DeserializeOwned + Sync + Send;
    const SERVICE_NAME: &'static str;

    fn get_request_client(&self) -> reqwest::RequestBuilder;

    async fn get_user(
        &self,
        db: &mut DbPoolConnection,
        user: Self::IntermediateUserInfo,
    ) -> Result<Option<User>>;

    async fn link_to(
        &self,
        db: &mut DbPoolConnection,
        user: &User,
        user_info: Self::IntermediateUserInfo,
    ) -> Result<bool>;

    async fn handle_link_callback(
        &self,
        db: &mut DbPoolConnection,
        user: &User,
    ) -> Result<Result<Redirect, Status>> {
        let user_info = self.fetch_user_info().await?;
        self.link_to(db, user, user_info).await.map(|linked| {
            if linked {
                Ok(
                    Message::success(&format!("Linked your account to {}", Self::SERVICE_NAME))
                        .to("/settings/account"),
                )
            } else {
                Ok(Message::error(&format!(
                    "This {} account is already linked to another WCPC account",
                    Self::SERVICE_NAME
                ))
                .to("/settings/account"))
            }
        })
    }

    async fn handle_callback(
        &self,
        db: &mut DbPoolConnection,
        cookies: &CookieJar<'_>,
    ) -> Result<Result<Redirect, Status>> {
        let user_info = self.fetch_user_info().await?;

        let db_conn = &mut *db;

        let user = self
            .get_user(db_conn, user_info)
            .await
            .with_context(|| format!("Failed to get user info from {}", Self::SERVICE_NAME))?;

        if let Some(user) = user {
            user.login(db_conn, cookies)
                .await
                .with_context(|| format!("Failed to login user from {}", Self::SERVICE_NAME))?;
            Ok(Ok(Redirect::to("/")))
        } else {
            Ok(Err(Status::Unauthorized))
        }
    }

    async fn fetch_user_info(&self) -> Result<Self::IntermediateUserInfo> {
        let resp = self
            .get_request_client()
            .send()
            .await
            .with_context(|| format!("Failed to send request to {}", Self::SERVICE_NAME))?;

        if resp.status().is_success() {
            let user_info = resp
                .json::<Self::IntermediateUserInfo>()
                .await
                .with_context(|| {
                    format!("Failed to parse user info from {}", Self::SERVICE_NAME)
                })?;
            Ok(user_info)
        } else {
            Err(anyhow!(
                "Failed to get user info from {}: {}",
                Self::SERVICE_NAME,
                resp.status()
            ))
        }
    }
}
