use super::project::Project;
use crate::project::Project as ProjectTrait;
use crate::task::{DateTimeUtc, Priority, State as TaskState, Task as TaskTrait};
use std::any::Any;
use std::fmt::{self, Write};

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Default)]
pub struct Task {
    pub root_path: String,
    pub provider: String,

    pub file_path: String,
    pub start_pos: usize,
    pub end_pos: usize,
    pub state: State,
    pub text: String,
    pub due: Option<DateTimeUtc>,
    pub completed_at: Option<DateTimeUtc>,
    pub priority: Priority,
}

impl PartialEq for Task {
    fn eq(&self, o: &Self) -> bool {
        self.start_pos == o.start_pos
            && self.end_pos == o.end_pos
            && self.state == o.state
            && self.text == o.text
            && self.due == o.due
            && self.priority == o.priority
    }
}

impl Eq for Task {}

impl Task {
    pub fn set_root_path(&mut self, p: String) {
        self.root_path = p;
    }
    pub fn set_provider(&mut self, p: String) {
        self.provider = p;
    }
}

impl TaskTrait for Task {
    fn text(&self) -> String {
        self.text.to_string()
    }

    fn state(&self) -> TaskState {
        match self.state {
            State::Completed => TaskState::Completed,
            State::Uncompleted => TaskState::Uncompleted,
            State::InProgress => TaskState::InProgress,
            State::Unknown(x) => TaskState::Unknown(x),
        }
    }

    fn place(&self) -> String {
        format!(
            "{}:{}",
            self.file_path
                .strip_prefix(self.root_path.as_str())
                .unwrap_or_default(),
            self.start_pos,
        )
    }

    fn due(&self) -> Option<DateTimeUtc> {
        self.due
    }

    fn completed_at(&self) -> Option<DateTimeUtc> {
        self.completed_at
    }

    fn provider(&self) -> String {
        self.provider.to_string()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        Some(Box::new(Project::new(
            &self.provider,
            &self.root_path,
            &self.file_path,
        )))
    }

    fn priority(&self) -> Priority {
        self.priority.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
}
