use std::collections::HashMap;

use markdown::{CompileOptions, Constructs, Options, ParseOptions};
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
    fields: Vec<String>,
    pub status: FormStatus,
}

impl FormTemplateObject {
    pub fn get(mut form: impl TemplatedForm) -> Self {
        let defaults = form.get_defaults();
        let keys = defaults.keys().cloned().collect::<Vec<_>>();
        FormTemplateObject {
            data: defaults,
            errors: HashMap::new(),
            status: FormStatus::None,
            fields: keys,
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
        let fields = data.keys().cloned().collect::<Vec<_>>();
        FormTemplateObject {
            data,
            errors,
            status,
            fields,
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
                        .unwrap_or_else(|| defaults.get(&name).cloned().unwrap_or_default()),
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

fn fake_attr(args: FunctionArgs) -> Result<Value, tera::Error> {
    let attr = args.get("attr").and_then(|o| o.as_str()).unwrap_or("");
    let val = args.get("val").and_then(|o| o.as_str()).unwrap_or("");
    Ok(tera::Value::String(format!("\"{attr}=\"{val}")))
}

fn render_markdown(args: FunctionArgs) -> Result<Value, tera::Error> {
    let text = args
        .get("md")
        .and_then(|o| o.as_str())
        .ok_or(tera::Error::msg("md not passed!"))?;
    let options = Options {
        parse: ParseOptions {
            constructs: Constructs {
                math_text: true,
                math_flow: true,
                ..Constructs::gfm()
            },
            ..ParseOptions::gfm()
        },
        compile: CompileOptions::gfm(),
    };

    let rendered = markdown::to_html_with_options(text, &options)
        .map_err(|e| tera::Error::msg(format!("Failed to render markdown: {:?}", e)))?;
    Ok(tera::Value::String(rendered))
}

fn len_of_form_data_list(args: FunctionArgs) -> Result<Value, tera::Error> {
    let data = args
        .get("data")
        .and_then(|o| o.as_array())
        .ok_or(tera::Error::msg("data not passed!"))
        .and_then(|v| {
            v.iter()
                .map(|s| {
                    s.as_str()
                        .ok_or(tera::Error::msg("data must be list of str!"))
                })
                .collect::<Result<Vec<&str>, _>>()
        })?;
    let list = args
        .get("list")
        .and_then(|s| s.as_str())
        .ok_or(tera::Error::msg("list not passed!"))?;

    let mut dat = data
        .into_iter()
        .filter_map(|name| {
            if name.starts_with(&format!("{list}[")) {
                Some(tera::Value::Number(
                    name[list.len() + 1..]
                        .split(']')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0)
                        .into(),
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    dat.dedup();

    Ok(tera::Value::Array(dat))
}

#[macro_export]
macro_rules! context_with_base {
    ($usr:expr, $($key:ident $(: $value:expr)?),*$(,)?) => {
        context! {
            logged_in: $usr.is_some(),
            user: $usr,
            name: $usr.map(|u| u.display_name()).unwrap_or_default(),
            version: env!("CARGO_PKG_VERSION"),
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
            version: env!("CARGO_PKG_VERSION"),
            $($key $(: $value)?),*
        }
    };
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Templating", |rocket| async {
        rocket.attach(Template::custom(|e| {
            e.tera.register_function("in_debug", in_debug);
            e.tera.register_function("gravatar", gravatar_function);
            e.tera.register_function("fake_attr", fake_attr);
            e.tera.register_function("render_markdown", render_markdown);
            e.tera
                .register_function("len_of_form_data_list", len_of_form_data_list);
        }))
    })
}
