use std::collections::HashMap;

use rocket::{fairing::AdHoc, form::Context, http::Status};
use rocket_dyn_templates::Template;
use tera::Value;

type FunctionArgs<'a> = &'a HashMap<String, Value>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormStatus {
    Success,
    Error,
    None,
}

pub trait TemplatedForm {
    fn get_defaults(&mut self) -> HashMap<String, String>;
}

#[derive(Debug, Serialize)]
pub struct FormTemplateObject {
    data: HashMap<String, String>,
    errors: HashMap<String, Vec<String>>,
    status: FormStatus,
}

impl FormTemplateObject {
    pub fn get(mut form: impl TemplatedForm) -> Self {
        FormTemplateObject {
            data: form.get_defaults(),
            errors: HashMap::new(),
            status: FormStatus::None,
        }
    }

    pub fn with_data(
        data: HashMap<String, String>,
        errors: HashMap<String, Vec<String>>,
        status: Status,
    ) -> Self {
        let status = match status.code {
            200 => FormStatus::Success,
            400 | 413 => FormStatus::Error,
            _ => FormStatus::None,
        };
        FormTemplateObject {
            data,
            errors,
            status,
        }
    }

    pub fn from_rocket_context(mut form: impl TemplatedForm, value: &Context<'_>) -> Self {
        let defaults = form.get_defaults();
        let data = value
            .fields()
            .map(|f| {
                let val = value.field_value(f);
                let name = f.to_string();
                (
                    name.clone(),
                    val.map(|s| s.to_string())
                        .and_then(|_| defaults.get(&name).cloned())
                        .unwrap_or_default(),
                )
            })
            .collect::<HashMap<_, _>>();
        let errors = value
            .fields()
            .map(|f| {
                let err = value
                    .field_errors(f)
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>();
                (f.to_string(), err)
            })
            .collect::<HashMap<_, _>>();
        Self::with_data(data, errors, value.status())
    }
}

fn in_debug(_: FunctionArgs) -> Result<Value, tera::Error> {
    Ok(tera::Value::Bool(cfg!(debug_assertions)))
}

pub fn gravatar_url(email: &str, size: u64) -> String {
    format!(
        "https://www.gravatar.com/avatar/{}?s={}&d=identicon&r=pg",
        sha256::digest(email),
        size
    )
}

fn gravatar_function(args: FunctionArgs) -> Result<Value, tera::Error> {
    let email = args.get("email").and_then(|o| o.as_str()).unwrap_or("");
    let size = args.get("size").and_then(|o| o.as_u64()).unwrap_or(30);
    Ok(tera::Value::String(gravatar_url(email, size)))
}

#[macro_export]
macro_rules! context_with_base {
    ($usr:expr, $($key:ident $(: $value:expr)?),*$(,)?) => {
        context! {
            logged_in: $usr.is_some(),
            user: $usr,
            name: $usr.map(|u| u.display_name()).unwrap_or_default(),
            $($key $(: $value)?),*
        }
    };
}

#[macro_export]
macro_rules! context_with_base_authed {
    ($usr:expr, $($key:ident $(: $value:expr)?),*$(,)?) => {
        context! {
            logged_in: true,
            user: $usr,
            name: $usr.display_name(),
            $($key $(: $value)?),*
        }
    };
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Templating", |rocket| async {
        rocket.attach(Template::custom(|e| {
            e.tera.register_function("in_debug", in_debug);
            e.tera.register_function("gravatar", gravatar_function);
        }))
    })
}
