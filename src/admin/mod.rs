use rocket::{fairing::AdHoc, get, routes, State};
use rocket_dyn_templates::Template;
use samael::service_provider::ServiceProvider;

use crate::{
    auth::{
        users::{Admin, User},
        SamlOptions, PREFERRED_SSO_BINDING,
    },
    context_with_base_authed,
    run::CodeInfo,
};

mod runs;
mod users;

#[get("/")]
async fn index(
    user: &User,
    _admin: &Admin,
    so: &State<SamlOptions>,
    sp: &State<ServiceProvider>,
    lang_config: &State<CodeInfo>,
) -> Template {
    let commit_hash = option_env!("GIT_COMMIT_HASH");
    let saml_options = so.inner();
    let idp_id = sp
        .inner()
        .idp_metadata
        .entity_id
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let sp_id = sp
        .inner()
        .entity_id
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let rustc_version = rustc_version_runtime::version().to_string();
    let idp_sso_binding = sp
        .sso_binding_location(PREFERRED_SSO_BINDING)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "Not Found".to_string());
    let run_config = &lang_config.run_config;
    let ctx = context_with_base_authed!(
        user,
        commit_hash,
        saml_options,
        idp_id,
        sp_id,
        idp_sso_binding,
        rustc_version,
        run_config
    );
    Template::render("admin", ctx)
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Admin", |rocket| async {
        rocket.mount(
            "/admin",
            routes![
                index,
                users::users,
                users::delete_user_get,
                users::delete_user_post,
                runs::runs,
                runs::cancel_run,
                runs::cancel_run_post,
                runs::cancel_all_runs,
                runs::cancel_all_runs_post,
            ],
        )
    })
}
