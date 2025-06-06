// SPDX-License-Identifier: MIT

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::task::{State, Task as TaskTrait};
use async_trait::async_trait;
use ratatui::style::Color;
use std::error::Error;

#[derive(Clone)]
pub enum DuePatchItem {
    Today,
    Tomorrow,
    ThisWeekend,
    NextWeek,
    NoDate,
}

pub struct TaskPatch {
    pub task: Box<dyn TaskTrait>,
    pub state: Option<State>,
    pub due: Option<DuePatchItem>,
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        self.state.is_none() && self.due.is_none()
    }

    pub fn is_task(&self, task: &dyn TaskTrait) -> bool {
        self.task.id() == task.id() && self.task.provider() == task.provider()
    }
}

pub struct PatchError {
    pub task: Box<dyn TaskTrait>,
    pub error: String,
}

impl PatchError {
    pub fn is_task(&self, task: &dyn TaskTrait) -> bool {
        self.task.id() == task.id() && self.task.provider() == task.provider()
    }
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error patching task with id {}: {}", self.task.id(), self.error)
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> String;
    fn type_name(&self) -> String;
    async fn tasks(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn Error>>;
    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>>;
    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError>;
    async fn reload(&mut self);
    fn color(&self) -> Color;
}
