// SPDX-License-Identifier: MIT

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::task::Task as TaskTrait;
use crate::task_patch::{PatchError, TaskPatch};
use async_trait::async_trait;
use ratatui::style::Color;
use std::error::Error;

pub struct GetTasksError {
    pub message: String,
}

impl From<Box<dyn Error>> for GetTasksError {
    fn from(e: Box<dyn Error>) -> Self {
        Self { message: e.to_string() }
    }
}

impl From<GetTasksError> for Box<dyn Error> {
    fn from(e: GetTasksError) -> Self {
        Box::<dyn Error>::from(e.message)
    }
}

impl std::fmt::Display for GetTasksError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[async_trait]
pub trait ProviderTrait: Send + Sync {
    fn name(&self) -> String;
    fn type_name(&self) -> String;
    async fn tasks(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, GetTasksError>;
    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>>;
    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError>;
    async fn reload(&mut self);
    fn color(&self) -> Color;
}
