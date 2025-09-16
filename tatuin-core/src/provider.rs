// SPDX-License-Identifier: MIT

use super::{
    StringError, filter,
    project::Project as ProjectTrait,
    task::{Priority, Task as TaskTrait},
    task_patch::{PatchError, TaskPatch},
};
use async_trait::async_trait;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Capabilities {
    pub create_task: bool,
}

#[async_trait]
pub trait ProviderTrait: Send + Sync + Debug {
    fn name(&self) -> String;
    fn type_name(&self) -> String;
    async fn tasks(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError>;
    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError>;
    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError>;
    async fn reload(&mut self);
    fn capabilities(&self) -> Capabilities;
    async fn create_task(&mut self, project_id: &str, tp: &TaskPatch) -> Result<(), StringError>;
    fn supported_priorities(&self) -> Vec<Priority> {
        Priority::values()
    }
}
