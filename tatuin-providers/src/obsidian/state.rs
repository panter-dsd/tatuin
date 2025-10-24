use std::fmt::Write;

use tatuin_core::task::State as TaskState;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    Unknown(char),
    Uncompleted,
    Completed,
    InProgress,
}

impl Default for State {
    fn default() -> Self {
        State::Unknown(' ')
    }
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

impl From<State> for char {
    fn from(st: State) -> Self {
        match st {
            State::Uncompleted => ' ',
            State::Completed => 'x',
            State::InProgress => '/',
            State::Unknown(x) => x,
        }
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            State::Completed => write!(f, "✅"),
            State::Uncompleted => write!(f, " "),
            State::InProgress => write!(f, "⏳"),
            State::Unknown(x) => f.write_char(*x),
        }
    }
}

impl From<TaskState> for State {
    fn from(v: TaskState) -> Self {
        match v {
            TaskState::Completed => State::Completed,
            TaskState::Uncompleted => State::Uncompleted,
            TaskState::InProgress => State::InProgress,
            TaskState::Unknown(x) => State::Unknown(x),
        }
    }
}
