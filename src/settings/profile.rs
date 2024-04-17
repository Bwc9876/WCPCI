#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use std::collections::HashMap;

use log::error;
use rocket::form::{Contextual, Form, FromForm};
use rocket::http::Status;
use rocket::{get, post};
use rocket_dyn_templates::Template;

use crate::template::{FormTemplateObject, TemplatedForm};
use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::User,
    },
    context_with_base_authed,
    db::DbConnection,
};

struct ProfileFormTemplate<'r> {
    user: &'r User,
}

impl TemplatedForm for ProfileFormTemplate<'_> {
    fn get_defaults(&mut self) -> HashMap<String, String> {
        HashMap::from_iter([
            ("bio".to_string(), self.user.bio.clone()),
            (
                "display_name".to_string(),
                self.user.display_name.as_deref().unwrap_or("").to_string(),
            ),
            (
                "profile_picture_source".to_string(),
                self.user.profile_picture_source.clone(),
            ),
        ])
    }
}

#[derive(FromForm)]
pub struct ProfileForm<'r> {
    #[field(validate = len(..=1024))]
    bio: &'r str,
    #[field(validate = len(..=32))]
    display_name: &'r str,
    #[field(validate = len(..=10))]
    profile_picture_source: &'r str,
}

#[get("/profile")]
pub fn profile_get(user: &User, _token: &CsrfToken) -> Template {
    let form_template = ProfileFormTemplate { user };
    let form = FormTemplateObject::get(form_template);
    let ctx = context_with_base_authed!(user, form);
    Template::render("settings/profile", ctx)
}

#[derive(Responder)]
pub enum SettingsProfilePostResponse {
    Template(Template),
    Error(Status),
}

#[post("/profile", data = "<form>")]
pub async fn profile_post(
    mut db: DbConnection,
    user: &User,
    _token: &VerifyCsrfToken,
    form: Form<Contextual<'_, ProfileForm<'_>>>,
) -> SettingsProfilePostResponse {
    let mut user = user.clone();
    if let Some(ref value) = form.value {
        let name = value.display_name.trim();
        let display_name = if name.is_empty() { None } else { Some(name) };
        user.display_name = display_name.map(|s| s.to_string());
        user.bio = value.bio.to_string();
        user.profile_picture_source = value.profile_picture_source.to_string();
        if value.profile_picture_source == "gravatar" || value.profile_picture_source == "github" {
            let res = sqlx::query!(
                "UPDATE user SET bio = ?, display_name = ?, profile_picture_source = ? WHERE id = ?",
                value.bio,
                display_name,
                value.profile_picture_source,
                user.id
            )
            .execute(&mut **db)
            .await;
            if let Err(why) = res {
                error!("Failed to update user {}: {:?}", user.id, why);
                return SettingsProfilePostResponse::Error(Status::InternalServerError);
            }
        } else {
            return SettingsProfilePostResponse::Error(Status::BadRequest);
        }
    };

    let form_template = ProfileFormTemplate { user: &user };
    let form = FormTemplateObject::from_rocket_context(form_template, &form.context);

    let ctx =
        context_with_base_authed!(&user, default_display_name: &user.default_display_name, form);

    SettingsProfilePostResponse::Template(Template::render("settings/profile", ctx))
}
