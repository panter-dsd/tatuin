// SPDX-License-Identifier: MIT

use tatuin_core::{
    task::{DateTimeUtc, Priority},
    task_patch::ValuePatch,
};

use super::{state::State, task::Task};

#[allow(dead_code)]
#[derive(Clone)]
pub struct TaskPatch<'a> {
    pub task: &'a Task,
    pub name: ValuePatch<String>,
    pub description: ValuePatch<String>,
    pub state: ValuePatch<State>,
    pub due: ValuePatch<DateTimeUtc>,
    pub scheduled: ValuePatch<DateTimeUtc>,
    pub priority: ValuePatch<Priority>,
}

pub struct PatchError {
    pub task: Task,
    pub error: String,
}
