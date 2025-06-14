use std::any::Any;

use chrono::Utc;

use super::project::Project as ProjectTrait;
use super::{
    task::{DateTimeUtc, Priority, State, Task as TaskTrait},
    task_patch::TaskPatch,
};

pub struct PatchedTask {
    task: Box<dyn TaskTrait>,
    patch: Option<TaskPatch>,
}

impl PatchedTask {
    pub fn new(task: Box<dyn TaskTrait>, patch: Option<TaskPatch>) -> Self {
        Self { task, patch }
    }
}

impl TaskTrait for PatchedTask {
    fn id(&self) -> String {
        self.task.id()
    }

    fn text(&self) -> String {
        self.task.text()
    }

    fn description(&self) -> Option<String> {
        self.task.description()
    }

    fn priority(&self) -> Priority {
        if let Some(p) = &self.patch {
            if let Some(v) = &p.priority {
                return v.clone();
            }
        }

        self.task.priority()
    }
    fn state(&self) -> State {
        if let Some(p) = &self.patch {
            if let Some(v) = &p.state {
                return v.clone();
            }
        }

        self.task.state()
    }
    fn created_at(&self) -> Option<DateTimeUtc> {
        self.task.created_at()
    }
    fn updated_at(&self) -> Option<DateTimeUtc> {
        self.task.updated_at()
    }
    fn completed_at(&self) -> Option<DateTimeUtc> {
        self.task.completed_at()
    }
    fn due(&self) -> Option<DateTimeUtc> {
        if let Some(p) = &self.patch {
            if let Some(v) = &p.due {
                return v.to_date(&Utc::now());
            }
        }

        self.task.due()
    }
    fn place(&self) -> String {
        self.task.place()
    }

    fn url(&self) -> String {
        self.task.url()
    }

    fn provider(&self) -> String {
        self.task.provider()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        self.task.project()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
}

impl Clone for PatchedTask {
    fn clone(&self) -> Self {
        Self {
            task: self.task.clone_boxed(),
            patch: self.patch.clone(),
        }
    }
}
