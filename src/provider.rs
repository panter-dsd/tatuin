// SPDX-License-Identifier: MIT

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::task::Task as TaskTrait;
use crate::task_patch::{PatchError, TaskPatch};
use async_trait::async_trait;
use ratatui::style::Color;
use std::error::Error;

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
