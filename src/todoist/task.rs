use crate::project::Project as ProjectTrait;
use crate::task::{DateTimeUtc, State as TaskState, Task as TaskTrait};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use std::any::Any;

use super::project::Project;

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Duration {
    property1: Option<String>,
    property2: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Due {
    date: String,
    timezone: Option<String>,
    string: String,
    lang: String,
    is_recurring: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Task {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub added_by_uid: Option<String>,
    pub assigned_by_uid: Option<String>,
    pub responsible_uid: Option<String>,
    pub labels: Option<Vec<String>>,
    pub deadline: Option<Duration>,
    pub duration: Option<Duration>,
    pub checked: Option<bool>,
    pub is_deleted: Option<bool>,
    pub added_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
    pub due: Option<Due>,
    pub priority: Option<i32>,
    pub child_order: Option<i32>,
    pub content: String,
    pub description: Option<String>,
    pub note_count: Option<i32>,
    pub day_order: Option<i32>,
    pub is_collapsed: Option<bool>,

    pub project: Option<Project>,
    pub provider: Option<String>,
}

fn str_to_date(s: &str) -> Option<DateTimeUtc> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0)?;
        return Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc));
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc));
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(DateTimeUtc::from(dt));
    }

    None
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn text(&self) -> String {
        self.content.to_string()
    }

    fn state(&self) -> TaskState {
        if self.checked.unwrap_or(true) {
            // completed tasks doesn't contain this field
            TaskState::Completed
        } else {
            TaskState::Uncompleted
        }
    }

    fn place(&self) -> String {
        if let Some(p) = &self.project {
            format!("project: {}", p.name)
        } else {
            "".to_string()
        }
    }

    fn due(&self) -> Option<DateTimeUtc> {
        let due = self.due.as_ref()?;

        str_to_date(due.date.as_str())
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        if let Some(s) = self.added_at.as_ref() {
            str_to_date(s.as_str())
        } else {
            None
        }
    }

    fn updated_at(&self) -> Option<DateTimeUtc> {
        if let Some(s) = self.updated_at.as_ref() {
            str_to_date(s.as_str())
        } else {
            None
        }
    }

    fn completed_at(&self) -> Option<DateTimeUtc> {
        if let Some(s) = self.completed_at.as_ref() {
            str_to_date(s.as_str())
        } else {
            None
        }
    }

    fn provider(&self) -> String {
        match &self.provider {
            Some(p) => p.to_string(),
            None => String::new(),
        }
    }
    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        if let Some(p) = &self.project {
            return Some(Box::new(p.clone()));
        }

        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
}
