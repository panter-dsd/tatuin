use crate::task;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Duration {
    property1: Option<String>,
    property2: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Due {
    date: String,
    timezone: Option<String>,
    string: String,
    lang: String,
    is_recurring: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Task {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub added_by_uid: Option<String>,
    pub assigned_by_uid: Option<String>,
    pub responsible_uid: Option<String>,
    pub labels: Vec<String>,
    pub deadline: Option<Duration>,
    pub duration: Option<Duration>,
    pub checked: bool,
    pub is_deleted: bool,
    pub added_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
    pub due: Option<Due>,
    pub priority: i32,
    pub child_order: i32,
    pub content: String,
    pub description: String,
    pub note_count: i32,
    pub day_order: i32,
    pub is_collapsed: bool,
}

impl task::Task for Task {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn text(&self) -> String {
        self.content.to_string()
    }

    fn state(&self) -> task::State {
        if self.checked {
            task::State::Completed
        } else {
            task::State::Uncompleted
        }
    }

    fn place(&self) -> String {
        self.id.to_string()
    }

    fn due(&self) -> Option<task::DateTimeUtc> {
        let due = self.due.as_ref()?;

        if let Ok(d) = NaiveDate::parse_from_str(due.date.as_str(), "%Y-%m-%d") {
            let dt = d.and_hms_opt(0, 0, 0)?;
            return Some(task::DateTimeUtc::from_naive_utc_and_offset(dt, Utc));
        }

        if let Ok(dt) = NaiveDateTime::parse_from_str(due.date.as_str(), "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(task::DateTimeUtc::from_naive_utc_and_offset(dt, Utc));
        }

        if let Ok(dt) = DateTime::parse_from_rfc3339(due.date.as_str()) {
            return Some(task::DateTimeUtc::from(dt));
        }

        None
    }
}
