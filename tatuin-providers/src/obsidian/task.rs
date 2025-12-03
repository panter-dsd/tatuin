// SPDX-License-Identifier: MIT

use crate::obsidian::internal_links_renderer::InternalLinksRenderer;

use super::{description::Description, fs, project::Project, state::State};
use std::{
    any::Any,
    path::{Path, PathBuf},
};
use tatuin_core::{
    RichStringTrait,
    project::Project as ProjectTrait,
    task::{DateTimeUtc, PatchPolicy, Priority, State as TaskState, Task as TaskTrait},
    task_patch::DuePatchItem,
};

#[derive(Debug, Clone, Default)]
pub struct Task {
    pub vault_path: PathBuf,
    pub provider: String,

    pub name: InternalLinksRenderer,
    pub file_path: PathBuf,
    pub start_pos: usize,
    pub end_pos: usize,
    pub state: State,
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
    pub fn set_vault_path(&mut self, p: &Path) {
        self.vault_path = p.to_path_buf();
        self.name.set_vault_path(p);
        self.name.remove_tags();
    }

    pub fn set_provider(&mut self, p: String) {
        self.provider = p;
    }
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        sha256::digest(format!(
            "{:?}:{}:{}:{}:{}",
            self.file_path,
            self.start_pos,
            self.end_pos,
            self.state,
            self.name.raw()
        ))
    }

    fn name(&self) -> Box<dyn RichStringTrait> {
        Box::new(self.name.clone())
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
            fs::strip_root_str(&self.vault_path, &self.file_path),
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
            &self.vault_path,
            &self.file_path,
        )))
    }

    fn priority(&self) -> Priority {
        self.priority
    }

    fn url(&self) -> String {
        fs::obsidian_url(&self.vault_path, &self.file_path)
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
