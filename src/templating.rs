// use log::error;
// use rocket::fairing::AdHoc;

// pub fn stage() -> AdHoc {
//     AdHoc::on_ignite("Tera Templates", |rocket| async {
//         let figment = rocket.figment();

//         let template_dir = figment
//             .find_value("template_dir")
//             .ok()
//             .and_then(|v| v.as_str().map(|s| s.to_string()))
//             .unwrap_or_else(|| "templates".to_string());

//         let ter = tera::Tera::new(&template_dir);

//         match ter {
//             Ok(tera) => rocket.manage(tera),
//             Err(e) => {
//                 error!("Failed to initialize Tera templates: {}", e);
//                 rocket
//             }
//         }
//     })
// }

// #[macro_export]
// macro_rules! render_template {
//     ($tera:expr, $filename:expr$(, {$($name:expr => $val:expr)*})?) => {
//         let mut context = tera::Context::new();
//         $(context.insert($name, $val);)*
//         $tera.render($filename, &context).map_err(|e| {
//             error!("Failed to render template {}: {}", $name, e);
//             e
//         })
//     };
// }
