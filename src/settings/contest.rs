#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use std::collections::HashMap;

use log::error;
use rocket::{
    form::{Contextual, Form},
    get, post, FromForm, State,
};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::CsrfToken,
        users::{ColorScheme, User},
    },
    context_with_base_authed,
    db::DbConnection,
    run::CodeInfo,
    template::{FormStatus, FormTemplateObject, TemplatedForm},
};

struct ContestFormTemplate<'r> {
    user: &'r User,
}

impl TemplatedForm for ContestFormTemplate<'_> {
    fn get_defaults(&mut self) -> HashMap<String, String> {
        HashMap::from_iter([
            (
                "color_scheme".to_string(),
                self.user.color_scheme.clone().into(),
            ),
            (
                "default_language".to_string(),
                self.user.default_language.clone(),
            ),
        ])
    }
}

#[get("/contest")]
pub fn contest_settings_get(
    user: &User,
    code_info: &State<CodeInfo>,
    _token: &CsrfToken,
) -> Template {
    let form_template = ContestFormTemplate { user };
    let form = FormTemplateObject::get(form_template);
    let languages = code_info
        .run_config
        .languages
        .iter()
        .map(|(k, l)| (k, &l.name))
        .collect::<Vec<_>>();
    let ctx = context_with_base_authed!(user, form, languages);
    Template::render("settings/contest", ctx)
}

#[derive(FromForm)]
pub struct ContestForm<'r> {
    color_scheme: ColorScheme,
    default_language: &'r str,
}

#[post("/contest", data = "<form>")]
pub async fn contest_settings_post(
    user: &User,
    mut form: Form<Contextual<'_, ContestForm<'_>>>,
    mut db: DbConnection,
    _token: &CsrfToken,
    code_info: &State<CodeInfo>,
) -> Template {
    let mut user = user.clone();
    let languages = code_info
        .run_config
        .languages
        .iter()
        .map(|(k, l)| (k, &l.name))
        .collect::<Vec<_>>();
    if let Some(ref value) = form.value {
        let default_language = value.default_language.trim();
        let color_scheme = &value.color_scheme;
        user.default_language = default_language.to_string();
        user.color_scheme = color_scheme.clone();
        if !code_info
            .run_config
            .languages
            .contains_key(default_language)
        {
            let error =
                rocket::form::Error::validation("Invalid language").with_name("default_language");
            let rocket_ctx = &mut form.context;
            rocket_ctx.push_error(error);
            let form_template = ContestFormTemplate { user: &user };
            let mut form_ctx = FormTemplateObject::from_rocket_context(form_template, rocket_ctx);
            form_ctx.status = FormStatus::Error;
            let ctx = context_with_base_authed!(&user, form: form_ctx, languages);
            return Template::render("settings/contest", ctx);
        } else {
            let res = sqlx::query!(
                "UPDATE user SET default_language = ?, color_scheme = ? WHERE id = ?",
                user.default_language,
                user.color_scheme,
                user.id
            )
            .execute(&mut **db)
            .await;
            if let Err(why) = res {
                error!("Failed to update user {}: {:?}", user.id, why);
            }
        }
    };

    let form_template = ContestFormTemplate { user: &user };
    let form = FormTemplateObject::from_rocket_context(form_template, &form.context);

    let ctx = context_with_base_authed!(&user, form, languages);

    Template::render("settings/contest", ctx)
}
