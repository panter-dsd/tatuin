use crate::filter;
use crate::project::Project as ProjectTrait;
use chrono::DateTime;
use chrono::prelude::*;
use colored::Colorize;
use std::any::Any;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Write;

pub type DateTimeUtc = DateTime<Utc>;

#[derive(Debug)]
pub enum State {
    Unknown(char),
    Uncompleted,
    Completed,
    InProgress,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            State::Completed => write!(f, "✅"),
            State::Uncompleted => write!(f, " "),
            State::InProgress => write!(f, "⏳"),
            State::Unknown(x) => f.write_char(*x),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Priority {
    Lowest,
    Low,
    #[default]
    Normal,
    Medium,
    High,
    Highest,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[allow(dead_code)]
pub trait Task: Send + Sync {
    fn id(&self) -> String {
        String::new()
    }

    fn text(&self) -> String {
        String::new()
    }

    fn description(&self) -> Option<String> {
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
    fn place(&self) -> String {
        String::new()
    }
    fn provider(&self) -> String;

    fn project(&self) -> Option<Box<dyn ProjectTrait>>;

    fn as_any(&self) -> &dyn Any;

    fn clone_boxed(&self) -> Box<dyn Task>;
}

pub fn datetime_to_str(t: Option<DateTimeUtc>) -> String {
    if let Some(d) = t {
        if d.time() == chrono::NaiveTime::default() {
            return d.format("%Y-%m-%d").to_string();
        }

        return d.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    String::from("-")
}

pub fn format(t: &dyn Task) -> String {
    format!(
        "- [{}] {} ({}) ({})",
        t.state(),
        t.text(),
        format!("due: {}", datetime_to_str(t.due())).blue(),
        t.place().green()
    )
}

pub fn due_group(t: &dyn Task) -> filter::Due {
    match t.due() {
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
