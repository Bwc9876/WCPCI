#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use std::collections::HashMap;

use rocket::{fairing::AdHoc, routes, FromForm};

mod cases;
mod completions;
mod edit;
mod new;
mod runs;
mod view;

pub use cases::TestCase;
pub use completions::ProblemCompletion;
pub use runs::JudgeRun;

use crate::{db::DbPoolConnection, template::TemplatedForm};

use self::cases::TestCaseForm;

#[derive(Serialize)]
pub struct Problem {
    pub id: i64,
    pub contest_id: i64,
    name: String,
    slug: String,
    description: String,
    pub cpu_time: i64,
}

impl Problem {
    pub async fn by_id(db: &mut DbPoolConnection, contest_id: i64, id: i64) -> Option<Self> {
        sqlx::query_as!(
            Problem,
            "SELECT * FROM problem WHERE id = ? AND contest_id = ?",
            id,
            contest_id
        )
        .fetch_optional(&mut **db)
        .await
        .ok()
        .flatten()
    }

    pub async fn get(db: &mut DbPoolConnection, contest_id: i64, slug: &str) -> Option<Self> {
        sqlx::query_as!(
            Problem,
            "SELECT * FROM problem WHERE contest_id = ? AND slug = ?",
            contest_id,
            slug
        )
        .fetch_optional(&mut **db)
        .await
        .ok()
        .flatten()
    }

    pub async fn list(db: &mut DbPoolConnection, contest_id: i64) -> Vec<Self> {
        sqlx::query_as!(
            Problem,
            "SELECT * FROM problem WHERE contest_id = ?",
            contest_id
        )
        .fetch_all(&mut **db)
        .await
        .unwrap()
    }

    pub async fn insert(&self, db: &mut DbPoolConnection) -> Result<Problem, sqlx::Error> {
        sqlx::query_as!(
            Problem,
            "INSERT INTO problem (name, contest_id, slug, description, cpu_time) VALUES (?, ?, ?, ?, ?) RETURNING *",
            self.name,
            self.contest_id,
            self.slug,
            self.description,
            self.cpu_time
        )
        .fetch_one(&mut **db)
        .await
    }

    pub async fn update(&self, db: &mut DbPoolConnection) -> Result<(), sqlx::Error> {
        sqlx::query_as!(
            Problem,
            "UPDATE problem SET name = ?, slug = ?, description = ?, cpu_time = ? WHERE id = ?",
            self.name,
            self.slug,
            self.description,
            self.cpu_time,
            self.id
        )
        .execute(&mut **db)
        .await
        .map(|_| ())
    }

    pub fn temp(contest_id: i64, form: &ProblemForm) -> Self {
        let slug = slug::slugify(form.name);
        Self {
            id: 0,
            contest_id,
            name: form.name.to_string(),
            slug,
            description: form.description.to_string(),
            cpu_time: form.cpu_time,
        }
    }
}

#[derive(FromForm)]
pub struct ProblemForm<'r> {
    #[field(validate = len(1..=32))]
    name: &'r str,
    description: &'r str,
    #[field(validate = range(1..=100))]
    cpu_time: i64,
    #[field(validate = len(..=50))]
    test_cases: Vec<TestCaseForm<'r>>,
}

pub struct ProblemFormTemplate<'r> {
    problem: Option<&'r Problem>,
    test_cases: Vec<TestCaseForm<'r>>,
}

impl<'r> TemplatedForm for ProblemFormTemplate<'r> {
    fn get_defaults(&mut self) -> HashMap<String, String> {
        if let Some(problem) = self.problem {
            let mut map = HashMap::from_iter([
                ("name".to_string(), problem.name.clone()),
                ("description".to_string(), problem.description.clone()),
                ("cpu_time".to_string(), problem.cpu_time.to_string()),
            ]);
            for (i, case) in self.test_cases.iter().enumerate() {
                map.insert(format!("test_cases[{}].stdin", i), case.stdin.to_string());
                map.insert(
                    format!("test_cases[{}].expected_pattern", i),
                    case.expected_pattern.to_string(),
                );
                map.insert(
                    format!("test_cases[{}].use_regex", i),
                    case.use_regex.to_string(),
                );
                map.insert(
                    format!("test_cases[{}].case_insensitive", i),
                    case.case_insensitive.to_string(),
                );
            }
            map
        } else {
            HashMap::from_iter([
                ("name".to_string(), "".to_string()),
                ("description".to_string(), "".to_string()),
                ("cpu_time".to_string(), "1".to_string()),
            ])
        }
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Problem Stage", |rocket| async {
        rocket.mount(
            "/contests",
            routes![
                view::list_problems_get,
                view::view_problem_get,
                new::new_problem_get,
                new::new_problem_post,
                edit::edit_problem_get,
                edit::edit_problem_post,
                runs::runs
            ],
        )
    })
}
