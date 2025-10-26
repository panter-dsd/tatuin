// SPDX-License-Identifier: MIT

use super::{description::Description, project::Project, state::State};
use std::{any::Any, path::PathBuf};
use tatuin_core::{
    project::Project as ProjectTrait,
    task::{DateTimeUtc, PatchPolicy, Priority, RawTaskName, State as TaskState, Task as TaskTrait, TaskNameProvider},
    task_patch::DuePatchItem,
};
use urlencoding::encode;

#[derive(Debug, Clone, Default)]
pub struct Task {
    pub root_path: String,
    pub provider: String,

    pub file_path: String,
    pub start_pos: usize,
    pub end_pos: usize,
    pub state: State,
    pub name: String,
    pub description: Option<Description>,
    pub due: Option<DateTimeUtc>,
    pub completed_at: Option<DateTimeUtc>,
    pub priority: Priority,
    pub tags: Vec<String>,
}

impl PartialEq for Task {
    fn eq(&self, o: &Self) -> bool {
        self.start_pos == o.start_pos
            && self.end_pos == o.end_pos
            && self.state == o.state
            && self.name == o.name
            && self.description == o.description
            && self.due == o.due
            && self.priority == o.priority
            && self.tags == o.tags
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
    fn id(&self) -> String {
        sha256::digest(format!(
            "{}:{}:{}:{}:{}",
            self.file_path, self.start_pos, self.end_pos, self.state, self.name
        ))
    }

    fn name(&self) -> Box<dyn TaskNameProvider> {
        Box::new(RawTaskName::from(&self.name))
    }

    fn description(&self) -> Option<String> {
        self.description.clone().map(|d| d.text)
    }

    fn state(&self) -> TaskState {
        self.state.into()
    }

    fn place(&self) -> String {
        format!(
            "{}:{}",
            self.file_path.strip_prefix(self.root_path.as_str()).unwrap_or_default(),
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
        Some(Box::new(Project::new(&self.provider, &self.root_path, &self.file_path)))
    }

    fn priority(&self) -> Priority {
        self.priority
    }

    fn url(&self) -> String {
        PathBuf::from(&self.root_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|vault_name| {
                format!(
                    "obsidian://open?vault={}&file={}",
                    vault_name,
                    encode(self.file_path.strip_prefix(self.root_path.as_str()).unwrap_or_default())
                )
            })
            .unwrap_or_default()
    }

    fn labels(&self) -> Vec<String> {
        self.tags.clone()
    }

    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            is_editable: true,
            is_removable: true,
            available_states: vec![TaskState::Uncompleted, TaskState::Completed, TaskState::InProgress],
            available_priorities: Priority::values(),
            available_due_items: DuePatchItem::values(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
}
