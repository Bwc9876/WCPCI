use std::path::PathBuf;

use rocket::{fairing::AdHoc, fs::FileServer};

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Static Asset Serving", |rocket| async {
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
        let public_dir = PathBuf::from(
            figment
                .find_value("public_dir")
                .ok()
                .and_then(|s| s.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "public".to_string()),
        );
        let path = template_dir.join("assets");
        let assets_dir = path.to_str().unwrap();
        rocket
            .mount("/_astro", FileServer::from(dir))
            .mount("/assets", FileServer::from(assets_dir))
            .mount("/", FileServer::from(public_dir).rank(15))
    })
}
