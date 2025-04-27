use chrono::DateTime;
use chrono::prelude::*;
use colored::Colorize;
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

#[allow(dead_code)]
pub trait Task: Send + Sync {
    fn id(&self) -> String {
        String::new()
    }
    fn text(&self) -> String {
        String::new()
    }
    fn priority(&self) -> i8 {
        0
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
}

pub fn due_to_str(t: Option<DateTimeUtc>) -> String {
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
        format!("due: {}", due_to_str(t.due())).blue(),
        t.place().green()
    )
}
