// SPDX-License-Identifier: MIT

use crate::task::{DateTimeUtc, Priority};

use super::task::{State, Task};

pub struct TaskPatch<'a> {
    pub task: &'a Task,
    pub name: Option<String>,
    pub state: Option<State>,
    pub due: Option<DateTimeUtc>,
    pub priority: Option<Priority>,
}

pub struct PatchError {
    pub task: Task,
    pub error: String,
}
