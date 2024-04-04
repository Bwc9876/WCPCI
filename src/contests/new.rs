use chrono::TimeZone;
use log::error;
use rocket::{
    form::{Contextual, Form},
    get, post,
    response::Redirect,
};
use rocket_dyn_templates::Template;

use crate::{
    auth::{
        csrf::{CsrfToken, VerifyCsrfToken},
        users::{Admin, User},
    },
    contests::ContestForm,
    context_with_base_authed,
    db::DbConnection,
    template::{FormStatus, FormTemplateObject},
    times::ClientTimeZone,
};

use super::{Contest, ContestFormTemplate};

#[get("/new")]
pub fn new_contest_get(
    user: &User,
    _admin: &Admin,
    timezone: ClientTimeZone,
    _token: &CsrfToken,
) -> Template {
    let form_template = ContestFormTemplate {
        contest: None,
        timezone: &timezone,
    };
    let form = FormTemplateObject::get(form_template);
    let ctx = context_with_base_authed!(user, form);
    Template::render("contests/new", ctx)
}

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum NewContestResponse {
    Template(Template),
    Redirect(Redirect),
}

#[post("/new", data = "<form>")]
pub async fn new_contest_post(
    mut db: DbConnection,
    user: &User,
    timezone: ClientTimeZone,
    _admin: &Admin,
    _token: &VerifyCsrfToken,
    form: Form<Contextual<'_, ContestForm<'_>>>,
) -> NewContestResponse {
    if let Some(ref value) = form.value {
        let tz = timezone.timezone();

        let name = value.name.to_string();
        let description = value.description.as_ref().map(|s| s.to_string());
        let start_time = tz
            .from_local_datetime(&value.start_time.0)
            .unwrap()
            .naive_utc();
        let registration_deadline = tz
            .from_local_datetime(&value.registration_deadline.0)
            .unwrap()
            .naive_utc();
        let end_time = tz
            .from_local_datetime(&value.end_time.0)
            .unwrap()
            .naive_utc();
        let freeze_time = value.freeze_time;
        let penalty = value.penalty;
        let max_participants = value.max_participants;
        let contest = Contest::temp(
            name,
            description,
            start_time,
            registration_deadline,
            end_time,
            freeze_time,
            penalty,
            max_participants,
        );
        if let Err(why) = contest.insert(&mut db).await {
            error!("Failed to insert contest: {}", why);
            let form_template = ContestFormTemplate {
                contest: Some(&contest),
                timezone: &timezone,
            };
            let mut form = FormTemplateObject::from_rocket_context(form_template, &form.context);
            form.status = FormStatus::Error;
            let ctx = context_with_base_authed!(user, form);
            NewContestResponse::Template(Template::render("contests/new", ctx))
        } else {
            NewContestResponse::Redirect(Redirect::to("/contests"))
        }
    } else {
        let form_template = ContestFormTemplate {
            contest: None,
            timezone: &timezone,
        };
        let form = FormTemplateObject::from_rocket_context(form_template, &form.context);
        let ctx = context_with_base_authed!(user, form);
        NewContestResponse::Template(Template::render("contests/new", ctx))
    }
}
