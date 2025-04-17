use crate::filter;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::prelude::*;
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
pub trait Task {
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

#[async_trait]
pub trait Provider {
    fn name(&self) -> String;
    async fn tasks(
        &self,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn Task>>, Box<dyn std::error::Error>>;
}
