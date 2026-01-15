// SPDX-License-Identifier: MIT

use crate::RichString;

use super::{filter, project::Project as ProjectTrait, task_patch::DatePatchItem};
use chrono::{DateTime, prelude::*};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    cmp::Ordering,
    fmt::{self, Write},
};

pub type DateTimeUtc = DateTime<Utc>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum State {
    Unknown(char),
    #[default]
    Uncompleted,
    Completed,
    InProgress,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            State::Completed => write!(f, "✅"),
            State::Uncompleted => write!(f, "  "),
            State::InProgress => write!(f, "⏳"),
            State::Unknown(x) => f.write_char(*x),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum Priority {
    Lowest,
    Low,
    #[default]
    Normal,
    Medium,
    High,
    Highest,
}

impl Priority {
    pub fn values() -> Vec<Priority> {
        vec![
            Priority::Lowest,
            Priority::Low,
            Priority::Normal,
            Priority::Medium,
            Priority::High,
            Priority::Highest,
        ]
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Default, Debug, Clone)]
pub struct PatchPolicy {
    pub is_editable: bool,
    pub is_removable: bool,
    pub available_states: Vec<State>,
    pub available_priorities: Vec<Priority>,
    pub available_due_items: Vec<DatePatchItem>,
    pub available_scheduled_items: Vec<DatePatchItem>,
}

#[allow(dead_code)]
pub trait Task: Send + Sync {
    fn id(&self) -> String;
    fn name(&self) -> RichString;

    fn description(&self) -> Option<RichString> {
        None
    }

    fn priority(&self) -> Priority {
        Priority::Normal
    }

    fn state(&self) -> State;
    fn created_at(&self) -> Option<DateTimeUtc> {
        None
    }
    fn updated_at(&self) -> Option<DateTimeUtc> {
        None
    }
    fn completed_at(&self) -> Option<DateTimeUtc> {
        None
    }
    fn due(&self) -> Option<DateTimeUtc> {
        None
    }
    fn scheduled(&self) -> Option<DateTimeUtc> {
        None
    }

    fn planned_date(&self) -> Option<DateTimeUtc> {
        *planned_date(&self.scheduled(), &self.due())
    }

    fn place(&self) -> String {
        String::new()
    }

    fn url(&self) -> String {
        String::new()
    }

    fn labels(&self) -> Vec<String> {
        Vec::new()
    }

    fn provider(&self) -> String;

    fn project(&self) -> Option<Box<dyn ProjectTrait>>;

    fn as_any(&self) -> &dyn Any;

    fn clone_boxed(&self) -> Box<dyn Task>;

    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            is_editable: false,
            is_removable: false,
            available_states: vec![State::Uncompleted, State::Completed, State::InProgress],
            available_priorities: Priority::values(),
            available_due_items: DatePatchItem::values(),
            available_scheduled_items: DatePatchItem::values(),
        }
    }

    fn patch_policy(&self) -> PatchPolicy {
        let mut pp = self.const_patch_policy();

        let s = self.state();
        pp.available_states.retain(|e| e != &s);
        let p = self.priority();
        pp.available_priorities.retain(|e| e != &p);
        pp
    }
}

pub fn datetime_to_str<Tz: TimeZone>(t: Option<DateTimeUtc>, tz: &Tz) -> String
where
    <Tz as TimeZone>::Offset: std::fmt::Display,
{
    if let Some(d) = t {
        if d.time() == chrono::NaiveTime::default() {
            return d.format("%Y-%m-%d").to_string();
        }

        return d.with_timezone(tz).format("%Y-%m-%d %H:%M:%S %Z").to_string();
    }

    String::from("-")
}

pub fn format(t: &dyn Task) -> String {
    format!(
        "- [{}] {} ({}) ({})",
        t.state(),
        t.name().display(),
        format!("due: {}", datetime_to_str(t.due(), &Local::now().timezone())).blue(),
        t.place().green()
    )
}

pub fn due_group(due: &Option<DateTimeUtc>) -> filter::Due {
    match due {
        Some(d) => {
            let now = chrono::Utc::now().date_naive();
            match d.date_naive().cmp(&now) {
                Ordering::Less => filter::Due::Overdue,
                Ordering::Equal => filter::Due::Today,
                Ordering::Greater => filter::Due::Future,
            }
        }
        None => filter::Due::NoDate,
    }
}

pub fn planned_date<'a>(scheduled: &'a Option<DateTimeUtc>, due: &'a Option<DateTimeUtc>) -> &'a Option<DateTimeUtc> {
    // TODO: check here if scheduled > due and return due in this case

    if scheduled.is_some() {
        return scheduled;
    }

    due
}
