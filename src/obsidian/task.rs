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
    pub file_path: String,
    pub pos: u64,
    pub state: State,
    pub text: String,
}
