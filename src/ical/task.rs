// SPDX-License-Identifier: MIT

use std::str::FromStr;

use chrono::{Duration, NaiveDate, NaiveDateTime};
use ical::property::Property;

use super::priority::TaskPriority;
use crate::{
    project::Project as ProjectTrait,
    task::{DateTimeUtc, PatchPolicy, Priority, State, Task as TaskTrait},
    task_patch::DuePatchItem,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TaskType {
    Event,
    #[default]
    Todo,
}

#[derive(strum::EnumString, strum::Display, Clone, Debug, Default, PartialEq, Eq)]
pub enum TaskStatus {
    #[strum(serialize = "TENTATIVE")]
    Tentative,
    #[strum(serialize = "CONFIRMED")]
    #[default]
    Confirmed,
    #[strum(serialize = "CANCELLED")]
    Cancelled,
    #[strum(serialize = "NEEDS-ACTION")]
    NeedsAction,
    #[strum(serialize = "COMPLETED")]
    Completed,
    #[strum(serialize = "IN-PROCESS")]
    InProcess,
    #[strum(serialize = "DRAFT")]
    Draft,
    #[strum(serialize = "FINAL")]
    Final,
}

impl From<TaskStatus> for State {
    fn from(value: TaskStatus) -> Self {
        match value {
            TaskStatus::Tentative | TaskStatus::Confirmed | TaskStatus::NeedsAction | TaskStatus::Draft => {
                State::Uncompleted
            }
            TaskStatus::Completed | TaskStatus::Final | TaskStatus::Cancelled => State::Completed,
            TaskStatus::InProcess => State::InProgress,
        }
    }
}

impl From<State> for TaskStatus {
    fn from(value: State) -> Self {
        match value {
            State::Unknown(_) | State::Uncompleted => TaskStatus::Confirmed,
            State::Completed => TaskStatus::Completed,
            State::InProgress => TaskStatus::InProcess,
        }
    }
}

#[derive(Default, Clone)]
pub struct Task {
    pub provider: String,
    pub properties: Vec<ical::property::Property>,
    pub href: String,
    pub task_type: TaskType,

    pub uid: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub status: TaskStatus,
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
            "Task uuid={} name={} description={:?} status={:?} priority={} start={:?} end={:?} due={:?} completed={:?} created={:?} duration={:?} categories={:?} properties={:?} href={} type={:?}",
            self.uid,
            self.name,
            self.description,
            self.status,
            self.priority,
            self.start,
            self.end,
            self.due,
            self.completed,
            self.created,
            self.duration,
            self.categories,
            self.properties,
            self.href,
            self.task_type,
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
        match self.status {
            TaskStatus::Tentative | TaskStatus::Confirmed | TaskStatus::NeedsAction | TaskStatus::Draft => {
                State::Uncompleted
            }
            TaskStatus::Completed | TaskStatus::Final | TaskStatus::Cancelled => State::Completed,
            TaskStatus::InProcess => State::InProgress,
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
            available_states: vec![State::Uncompleted, State::InProgress, State::Completed],
            available_priorities: Priority::values(),
            available_due_items: DuePatchItem::values(),
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
                "STATUS" => {
                    t.status = p
                        .value
                        .as_ref()
                        .map(|s| TaskStatus::from_str(s).unwrap_or_default())
                        .unwrap_or_default()
                }
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

pub fn property_to_str(value: &Property) -> String {
    format!("{}:{}", value.name, value.value.as_ref().unwrap_or(&String::new()))
}

fn make_property(name: &str, value: Option<String>) -> Property {
    Property {
        name: name.to_string(),
        params: None,
        value,
    }
}

fn replace_or_add(properties: &mut Vec<Property>, p: Property) {
    properties.retain(|prop| prop.name != p.name);
    if p.value.as_ref().is_some_and(|s| !s.is_empty()) {
        properties.push(p);
    }
}

impl From<&Task> for Vec<Property> {
    fn from(t: &Task) -> Self {
        const DT_FORMAT: &str = "%Y%m%dT%H%M%SZ";
        let mut result = t.properties.clone();
        replace_or_add(&mut result, make_property("SUMMARY", Some(t.name.clone())));
        replace_or_add(&mut result, make_property("DESCRIPTION", t.description.clone()));
        replace_or_add(&mut result, make_property("STATUS", Some(t.status.to_string())));
        replace_or_add(&mut result, make_property("PRIORITY", Some(t.priority.to_string())));
        replace_or_add(
            &mut result,
            make_property("DUE", t.due.map(|d| d.format(DT_FORMAT).to_string())),
        );
        replace_or_add(
            &mut result,
            make_property("CREATED", Some(chrono::Utc::now().format(DT_FORMAT).to_string())),
        );
        replace_or_add(
            &mut result,
            make_property("DTSTART", t.start.map(|d| d.format(DT_FORMAT).to_string())),
        );
        replace_or_add(
            &mut result,
            make_property("DTEND", t.end.map(|d| d.format(DT_FORMAT).to_string())),
        );
        result
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
