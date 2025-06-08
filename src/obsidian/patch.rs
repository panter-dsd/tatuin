use crate::task::DateTimeUtc;

use super::task::{State, Task};

pub struct TaskPatch<'a> {
    pub task: &'a Task,
    pub state: Option<State>,
    pub due: Option<DateTimeUtc>,
}

pub struct PatchError {
    pub task: Task,
    pub error: String,
}
