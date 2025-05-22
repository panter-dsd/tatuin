// SPDX-License-Identifier: MIT

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::task::{State, Task as TaskTrait};
use async_trait::async_trait;
use ratatui::style::Color;
use std::error::Error;

pub struct TaskPatch {
    pub task: Box<dyn TaskTrait>,
    pub state: Option<State>,
}

pub struct PatchError {
    pub task: Box<dyn TaskTrait>,
    pub error: String,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error patching task with id {}: {}", self.task.id(), self.error)
    }
}

#[async_trait]
pub trait Provider: Send {
    fn name(&self) -> String;
    fn type_name(&self) -> String;
    async fn tasks(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn Error>>;
    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>>;
    async fn change_task_state(&mut self, task: &dyn TaskTrait, state: State) -> Result<(), Box<dyn Error>>;
    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut errors = Vec::new();

        for patch in patches.iter() {
            if let Some(s) = &patch.state {
                if let Err(e) = self.change_task_state(patch.task.as_ref(), s.clone()).await {
                    errors.push(PatchError {
                        task: patch.task.clone_boxed(),
                        error: e.to_string(),
                    });
                }
            }
        }

        errors
    }
    async fn reload(&mut self);
    fn color(&self) -> Color;
}
