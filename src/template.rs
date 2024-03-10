use std::collections::HashMap;

use rocket::fairing::AdHoc;
use rocket_dyn_templates::Template;
use tera::Value;

type FunctionArgs<'a> = &'a HashMap<String, Value>;

fn in_debug(_: FunctionArgs) -> Result<Value, tera::Error> {
    Ok(tera::Value::Bool(cfg!(debug_assertions)))
}

#[macro_export]
macro_rules! context_with_base {
    ($usr:expr, $($key:ident $(: $value:expr)?),*$(,)?) => {
        context! {
            logged_in: $usr.is_some(),
            name: $usr.map(|u| u.display_name().to_string()),
            $($key $(: $value)?),*
        }
    };
}

#[macro_export]
macro_rules! context_with_base_authed {
    ($usr:expr, $($key:ident $(: $value:expr)?),*$(,)?) => {
        context_with_base! {
            Some($usr),
            $($key $(: $value)?),*
        }
    };
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Templating", |rocket| async {
        rocket.attach(Template::custom(|e| {
            e.tera.register_function("in_debug", in_debug);
        }))
    })
}
