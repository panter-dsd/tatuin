use std::fmt;
use std::fmt::Write;
use time::PrimitiveDateTime;

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
    fn created_at(&self) -> Option<PrimitiveDateTime> {
        None
    }
    fn updated_at(&self) -> Option<PrimitiveDateTime> {
        None
    }
    fn completed_at(&self) -> Option<PrimitiveDateTime> {
        None
    }
    fn due(&self) -> Option<PrimitiveDateTime> {
        None
    }
}
