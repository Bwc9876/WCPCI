use rocket::{fairing::AdHoc, routes};

mod account;
mod contest;
mod profile;

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Settings App", |rocket| async {
        rocket.mount(
            "/settings",
            routes![
                profile::profile_get,
                profile::profile_post,
                account::account_get,
                contest::contest_settings_get,
                contest::contest_settings_post,
            ],
        )
    })
}
