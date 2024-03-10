use std::path::PathBuf;

use rocket::{fairing::AdHoc, fs::FileServer};

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Setup Static Serving", |rocket| async {
        let figment = rocket.figment();
        let template_dir = PathBuf::from(
            figment
                .find_value("template_dir")
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "templates".to_string()),
        );
        let path = template_dir.join("_astro");
        let dir = path.to_str().unwrap();
        rocket.mount("/_astro", FileServer::from(dir))
    })
}
