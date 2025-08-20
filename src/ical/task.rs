// SPDX-License-Identifier: MIT

use chrono::Duration;

use crate::project::Project as ProjectTrait;
use crate::task::{DateTimeUtc, PatchPolicy, Priority, State, Task as TaskTrait};

#[derive(Default, Debug, Clone)]
pub struct Task {
    pub provider: String,

    pub uid: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: u8,
    pub start: Option<DateTimeUtc>,
    pub end: Option<DateTimeUtc>,
    pub due: Option<DateTimeUtc>,
    pub completed: Option<DateTimeUtc>,
    pub created: Option<DateTimeUtc>,
    pub duration: Option<Duration>,
    pub categories: Vec<String>,
}

impl Task {
    pub fn is_valid(&self) -> bool {
        !self.uid.is_empty() && !self.name.is_empty()
    }

    pub fn set_provider(&mut self, p: &str) {
        self.provider = p.to_string();
    }
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        self.uid.clone()
    }

    fn text(&self) -> String {
        self.name.clone()
    }

    fn state(&self) -> State {
        if self.completed.is_some() {
            State::Completed
        } else {
            State::Uncompleted
        }
    }

    fn provider(&self) -> String {
        self.provider.clone()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        None
    }

    fn due(&self) -> Option<DateTimeUtc> {
        if self.due.is_some() { self.due } else { self.start }
    }

    fn completed_at(&self) -> Option<DateTimeUtc> {
        self.completed
    }

    fn priority(&self) -> Priority {
        match self.priority {
            0 | 5 => Priority::Normal,
            1 => Priority::Highest,
            2 | 3 => Priority::High,
            4 => Priority::Medium,
            6 | 7 => Priority::Low,
            8.. => Priority::Lowest,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }

    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            available_states: Vec::new(),
            available_priorities: Vec::new(),
            available_due_items: Vec::new(),
        }
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        self.created
    }

    fn place(&self) -> String {
        self.provider()
    }

    fn labels(&self) -> Vec<String> {
        self.categories.clone()
    }
}
