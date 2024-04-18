#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use log::{error, warn};
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
    ) -> Result<Option<User>, String>;

    async fn link_to(
        &self,
        db: &mut DbPoolConnection,
        user: &User,
        user_info: Self::IntermediateUserInfo,
    ) -> Result<bool, String>;

    async fn handle_link_callback(
        &self,
        db: &mut DbPoolConnection,
        user: &User,
    ) -> Result<Result<Redirect, Status>, String> {
        let user_info = self.fetch_user_info().await?;
        self.link_to(db, user, user_info).await.map(|linked| {
            if linked {
                Ok(Redirect::to("/settings/account"))
            } else {
                Err(Status::Conflict)
            }
        })
    }

    async fn handle_callback(
        &self,
        db: &mut DbPoolConnection,
        cookies: &CookieJar<'_>,
    ) -> Result<Result<Redirect, Status>, String> {
        let user_info = self.fetch_user_info().await?;

        let db_conn = &mut *db;

        let user = self
            .get_user(db_conn, user_info)
            .await
            .map_err(|e| format!("Failed to get user info from {}: {e:?}", Self::SERVICE_NAME))?;

        if let Some(user) = user {
            user.login(db_conn, cookies)
                .await
                .map_err(|e| format!("Failed to login user from {}: {e:?}", Self::SERVICE_NAME))?;
            Ok(Ok(Redirect::to("/")))
        } else {
            Ok(Err(Status::Unauthorized))
        }
    }

    async fn fetch_user_info(&self) -> Result<Self::IntermediateUserInfo, String> {
        let resp = self
            .get_request_client()
            .send()
            .await
            .map_err(|e| format!("Failed to send request to {}: {e:?}", Self::SERVICE_NAME))?;

        if resp.status().is_success() {
            let user_info = resp
                .json::<Self::IntermediateUserInfo>()
                .await
                .map_err(|e| {
                    format!(
                        "Failed to parse user info from {}: {e:?}",
                        Self::SERVICE_NAME
                    )
                })?;
            Ok(user_info)
        } else {
            Err(format!(
                "Failed to get user info from {}: {}",
                Self::SERVICE_NAME,
                resp.status()
            ))
        }
    }
}
