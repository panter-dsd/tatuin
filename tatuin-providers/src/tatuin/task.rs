// SPDX-License-Identifier: MIT

use super::project::Project;
use redb::Value;
use serde::{Deserialize, Serialize};
use tatuin_core::{
    RichString,
    project::Project as ProjectTrait,
    task::{DateTimeUtc, PatchPolicy, Priority, State, Task as TaskTrait},
    task_patch::DuePatchItem,
};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Task {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub state: State,
    pub priority: Priority,
    pub labels: Vec<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub completed_at: Option<DateTimeUtc>,
    pub due: Option<DateTimeUtc>,
    pub project_id: uuid::Uuid,

    #[serde(skip_serializing, skip_deserializing)]
    provider: String,
    #[serde(skip_serializing, skip_deserializing)]
    project: Option<Project>,
}

impl Task {
    pub fn set_provider(&mut self, name: &str) {
        self.provider = name.to_string()
    }

    pub fn set_project(&mut self, p: Project) {
        self.project = Some(p)
    }
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn name(&self) -> RichString {
        RichString::new(&self.name)
    }

    fn description(&self) -> Option<RichString> {
        self.description.as_ref().map(|s| RichString::new(s))
    }

    fn priority(&self) -> Priority {
        self.priority
    }

    fn state(&self) -> State {
        self.state
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        Some(self.created_at)
    }

    fn updated_at(&self) -> Option<DateTimeUtc> {
        Some(self.updated_at)
    }

    fn completed_at(&self) -> Option<DateTimeUtc> {
        self.completed_at
    }

    fn due(&self) -> Option<DateTimeUtc> {
        self.due
    }

    fn place(&self) -> String {
        "db".to_string()
    }

    fn labels(&self) -> Vec<String> {
        self.labels.clone()
    }

    fn provider(&self) -> String {
        self.provider.clone()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        self.project.as_ref().map(|p| p.clone_boxed())
    }

    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            is_editable: true,
            is_removable: true,
            available_states: vec![State::Uncompleted, State::Completed, State::InProgress],
            available_priorities: Priority::values(),
            available_due_items: DuePatchItem::values(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
}

impl Value for Task {
    type SelfType<'a>
        = Task
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Vec<u8>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Task")
    }
}
