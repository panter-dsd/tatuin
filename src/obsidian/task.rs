use crate::task;
use chrono::prelude::*;
use std::fmt::{self, Write};

#[derive(Debug)]
pub enum State {
    Unknown(char),
    Uncompleted,
    Completed,
    InProgress,
}

impl State {
    pub fn new(c: char) -> Self {
        match c {
            ' ' => State::Uncompleted,
            'x' => State::Completed,
            '/' => State::InProgress,
            _ => State::Unknown(c),
        }
    }
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

#[derive(Debug)]
pub struct Task {
    pub root_path: String,
    pub file_path: String,
    pub pos: u64,
    pub state: State,
    pub text: String,
    pub due: Option<task::DateTimeUtc>,
}

impl Task {
    pub fn set_root_path(&mut self, p: String) {
        self.root_path = p;
    }
}

impl task::Task for Task {
    fn text(&self) -> String {
        self.text.to_string()
    }

    fn state(&self) -> task::State {
        match self.state {
            State::Completed => task::State::Completed,
            State::Uncompleted => task::State::Uncompleted,
            State::InProgress => task::State::InProgress,
            State::Unknown(x) => task::State::Unknown(x),
        }
    }

    fn place(&self) -> String {
        format!(
            "{}:{}",
            self.file_path
                .strip_prefix(self.root_path.as_str())
                .unwrap_or_default(),
            self.pos,
        )
    }

    fn due(&self) -> Option<task::DateTimeUtc> {
        self.due
    }
}
