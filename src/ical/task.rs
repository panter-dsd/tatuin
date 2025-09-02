// SPDX-License-Identifier: MIT

use chrono::{Duration, NaiveDate, NaiveDateTime};
use ical::property::Property;

use super::priority::TaskPriority;
use crate::{
    project::Project as ProjectTrait,
    task::{DateTimeUtc, PatchPolicy, Priority, State, Task as TaskTrait},
};

#[derive(Default, Clone)]
pub struct Task {
    pub provider: String,
    pub properties: Vec<ical::property::Property>,

    pub uid: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub start: Option<DateTimeUtc>,
    pub end: Option<DateTimeUtc>,
    pub due: Option<DateTimeUtc>,
    pub completed: Option<DateTimeUtc>,
    pub created: Option<DateTimeUtc>,
    pub duration: Option<Duration>,
    pub categories: Vec<String>,
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Task uuid={} name={} description={:?} priority={} start={:?} end={:?} due={:?} completed={:?} created={:?} duration={:?} categories={:?} properties={:?}",
            self.uid,
            self.name,
            self.description,
            self.priority,
            self.start,
            self.end,
            self.due,
            self.completed,
            self.created,
            self.duration,
            self.categories,
            self.properties
        )
    }
}

impl Task {
    pub fn is_valid(&self) -> bool {
        !self.uid.is_empty() && !self.name.is_empty()
    }

    pub fn set_provider(&mut self, p: &str) {
        self.provider = p.to_string();
    }
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        self.uid.clone()
    }

    fn text(&self) -> String {
        self.name.clone()
    }

    fn state(&self) -> State {
        if self.completed.is_some() {
            State::Completed
        } else {
            State::Uncompleted
        }
    }

    fn provider(&self) -> String {
        self.provider.clone()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        None
    }

    fn due(&self) -> Option<DateTimeUtc> {
        if self.due.is_some() { self.due } else { self.start }
    }

    fn completed_at(&self) -> Option<DateTimeUtc> {
        self.completed
    }

    fn priority(&self) -> Priority {
        self.priority.into()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }

    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            available_states: Vec::new(),
            available_priorities: Vec::new(),
            available_due_items: Vec::new(),
        }
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        self.created
    }

    fn place(&self) -> String {
        self.provider()
    }

    fn labels(&self) -> Vec<String> {
        self.categories.clone()
    }
}

impl From<&Vec<Property>> for Task {
    fn from(properties: &Vec<Property>) -> Self {
        let mut t = Task {
            properties: properties.to_vec(),
            ..Task::default()
        };

        for p in properties {
            match p.name.as_str() {
                "UID" => t.uid = p.value.clone().unwrap_or_default(),
                "SUMMARY" => t.name = p.value.clone().unwrap_or_default(),
                "DESCRIPTION" => t.description = p.value.clone(),
                "PRIORITY" => t.priority = p.value.as_ref().map(|s| s.parse::<u8>().unwrap_or(0)).unwrap_or(0),
                "DUE" => t.due = dt_from_property(p),
                "DTSTART" => t.start = dt_from_property(p),
                "DTEND" => t.end = dt_from_property(p),
                "COMPLETED" => t.completed = dt_from_property(p),
                "CREATED" => t.created = dt_from_property(p),
                "DURATION" => t.duration = duration_from_property(p),
                "CATEGORIES" if p.value.is_some() => {
                    t.categories = p
                        .value
                        .as_ref()
                        .unwrap()
                        .split(",")
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                }
                _ => {}
            }
        }
        tracing::debug!(task=?t, "New task");
        t
    }
}

fn tz_offset_from_property_params(params: &Option<Vec<(String, Vec<String>)>>) -> Option<chrono_tz::Tz> {
    if let Some(params) = params {
        for (n, p) in params {
            if n == "TZID"
                && p.len() == 1
                && let Ok(t) = p[0].parse::<chrono_tz::Tz>()
            {
                return Some(t);
            }
        }
    }

    None
}

fn dt_from_property(p: &Property) -> Option<DateTimeUtc> {
    let s = p.value.as_ref()?;

    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
        let dt = d.and_hms_opt(0, 0, 0)?;
        return Some(DateTimeUtc::from_naive_utc_and_offset(dt, chrono::Utc));
    }

    // with timezone in params
    if let Some(tz) = tz_offset_from_property_params(&p.params)
        && let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S")
    {
        let dt = dt.and_local_timezone(tz).unwrap();
        return Some(dt.to_utc());
    }

    // with timezone inside
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%SZ") {
        return Some(DateTimeUtc::from_naive_utc_and_offset(dt, chrono::Utc));
    }

    None
}

fn duration_from_property(p: &Property) -> Option<Duration> {
    if let Some(v) = &p.value
        && let Ok(d) = v.parse::<iso8601_duration::Duration>()
    {
        return d.to_chrono();
    }

    None
}
