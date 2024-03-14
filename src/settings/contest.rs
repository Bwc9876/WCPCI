use std::collections::HashMap;

use rocket::get;
use rocket_dyn_templates::Template;

use crate::{
    auth::users::User,
    context_with_base_authed,
    template::{FormTemplateObject, TemplatedForm},
};

struct ContestFormTemplate {}

impl TemplatedForm for ContestFormTemplate {
    fn get_defaults(&mut self) -> HashMap<String, String> {
        HashMap::new()
    }
}

#[get("/contest")]
pub fn contest_settings_get(user: &User) -> Template {
    let form_template = ContestFormTemplate {};
    let form = FormTemplateObject::get(form_template);
    let ctx = context_with_base_authed!(user, form);
    Template::render("settings/contest", ctx)
}
