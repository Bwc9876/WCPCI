#![allow(clippy::blocks_in_conditions)] // Needed for the derive of FromForm, rocket is weird

use std::collections::HashMap;

use chrono::{NaiveDateTime, TimeZone};
use rocket::{fairing::AdHoc, form, routes, FromForm};
use serde::Serialize;

use crate::{
    db::DbPoolConnection,
    template::TemplatedForm,
    times::{datetime_to_html_time, ClientTimeZone, FormDateTime},
};

mod edit;
mod join;
mod list;
mod new;
mod participant;
mod view;

pub use participant::Participant;

#[derive(Serialize)]
pub struct Contest {
    id: i64,
    pub name: String,
    description: Option<String>,
    #[serde(serialize_with = "crate::times::serialize_to_js")]
    start_time: NaiveDateTime,
    #[serde(serialize_with = "crate::times::serialize_to_js")]
    registration_deadline: NaiveDateTime,
    #[serde(serialize_with = "crate::times::serialize_to_js")]
    end_time: NaiveDateTime,
    max_participants: Option<i64>,
    created_at: Option<NaiveDateTime>,
}

impl Contest {
    pub fn temp(
        name: String,
        description: Option<String>,
        start_time: NaiveDateTime,
        registration_deadline: NaiveDateTime,
        end_time: NaiveDateTime,
        max_participants: Option<i64>,
    ) -> Self {
        Self {
            id: 0,
            name,
            description,
            start_time,
            registration_deadline,
            end_time,
            max_participants,
            created_at: None,
        }
    }

    pub async fn list(db: &mut DbPoolConnection) -> Vec<Self> {
        sqlx::query_as!(Contest, "SELECT * FROM contest")
            .fetch_all(&mut **db)
            .await
            .unwrap_or(vec![])
    }

    pub async fn get(db: &mut DbPoolConnection, id: i64) -> Option<Self> {
        sqlx::query_as!(Contest, "SELECT * FROM contest WHERE id = ?", id)
            .fetch_optional(&mut **db)
            .await
            .ok()
            .flatten()
    }

    pub async fn insert(&self, db: &mut DbPoolConnection) -> Result<Contest, sqlx::Error> {
        sqlx::query_as!(
            Contest,
            "INSERT INTO contest (name, description, start_time, registration_deadline, end_time, max_participants) VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
            self.name,
            self.description,
            self.start_time,
            self.registration_deadline,
            self.end_time,
            self.max_participants
        ).fetch_one(&mut **db).await
    }

    pub async fn update(&self, db: &mut DbPoolConnection) -> Result<(), sqlx::Error> {
        sqlx::query_as!(
            Contest,
            "UPDATE contest SET name = ?, description = ?, start_time = ?, registration_deadline = ?, end_time = ?, max_participants = ? WHERE id = ?",
            self.name,
            self.description,
            self.start_time,
            self.registration_deadline,
            self.end_time,
            self.max_participants,
            self.id
        ).execute(&mut **db).await.map(|_| ())
    }

    pub fn has_started(&self) -> bool {
        let now = chrono::offset::Utc::now().naive_utc();
        self.start_time < now
    }

    pub fn is_running(&self) -> bool {
        let now = chrono::offset::Utc::now().naive_utc();
        println!(
            "Start: {}, End: {}, Now: {}",
            self.start_time, self.end_time, now
        );
        self.start_time < now && self.end_time > now
    }

    pub fn can_register(&self) -> bool {
        let now = chrono::offset::Utc::now().naive_utc();
        self.registration_deadline > now
    }
}

struct ContestFormTemplate<'r> {
    contest: Option<&'r Contest>,
    timezone: &'r ClientTimeZone,
}

impl<'r> TemplatedForm for ContestFormTemplate<'r> {
    fn get_defaults(&mut self) -> std::collections::HashMap<String, String> {
        if let Some(contest) = self.contest {
            HashMap::from_iter([
                ("name".to_string(), contest.name.to_string()),
                (
                    "description".to_string(),
                    contest
                        .description
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "".to_string()),
                ),
                (
                    "start_time".to_string(),
                    datetime_to_html_time(
                        &self
                            .timezone
                            .timezone()
                            .from_utc_datetime(&contest.start_time),
                    ),
                ),
                (
                    "registration_deadline".to_string(),
                    datetime_to_html_time(
                        &self
                            .timezone
                            .timezone()
                            .from_utc_datetime(&contest.registration_deadline),
                    ),
                ),
                (
                    "end_time".to_string(),
                    datetime_to_html_time(
                        &self
                            .timezone
                            .timezone()
                            .from_utc_datetime(&contest.end_time),
                    ),
                ),
                (
                    "max_participants".to_string(),
                    contest
                        .max_participants
                        .map(|i| i.to_string())
                        .unwrap_or("null".to_string()),
                ),
            ])
        } else {
            HashMap::from_iter([
                ("name".to_string(), "".to_string()),
                ("description".to_string(), "".to_string()),
                ("start_time".to_string(), String::new()),
                ("registration_deadline".to_string(), String::new()),
                ("end_time".to_string(), String::new()),
                ("max_participants".to_string(), "".to_string()),
            ])
        }
    }
}

fn over_1<'e>(max_participants: &Option<i64>) -> Result<(), rocket::form::Errors<'e>> {
    if let Some(i) = max_participants {
        if *i > 0 {
            Ok(())
        } else {
            Err(form::Error::validation("Must be over 1").into())
        }
    } else {
        Ok(())
    }
}

fn len_under_1000<'r, 'e>(s: &'r Option<&'r str>) -> Result<(), rocket::form::Errors<'e>> {
    if let Some(s) = s {
        if s.len() < 1000 {
            Ok(())
        } else {
            Err(form::Error::validation("Must be under 1000 characters").into())
        }
    } else {
        Ok(())
    }
}

#[derive(FromForm)]
struct ContestForm<'r> {
    #[field(validate = len(1..=100))]
    name: &'r str,
    #[field(validate = len_under_1000())]
    description: Option<&'r str>,
    start_time: FormDateTime,
    registration_deadline: FormDateTime,
    end_time: FormDateTime,
    #[field(validate = over_1())]
    max_participants: Option<i64>,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Contests App", |rocket| async {
        rocket.mount(
            "/contests",
            routes![
                list::contests_list,
                new::new_contest_get,
                new::new_contest_post,
                edit::edit_contest_get,
                edit::edit_contest_post,
                join::join_contest,
                view::view_contest,
            ],
        )
    })
}
