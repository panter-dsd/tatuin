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
pub trait TaskProviderTrait {
    async fn list(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError>;
    async fn create(&mut self, project_id: &str, tp: &TaskPatch) -> Result<(), StringError>;
    async fn update(&mut self, patches: &[TaskPatch]) -> Vec<PatchError>;
    async fn delete(&mut self, _t: &dyn TaskTrait) -> Result<(), StringError> {
        unimplemented!()
    }
}

#[async_trait]
pub trait ProjectProviderTrait {
    async fn list(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError>;
}

#[async_trait]
pub trait ProviderTrait: TaskProviderTrait + ProjectProviderTrait + Send + Sync + Debug {
    fn name(&self) -> String;
    fn type_name(&self) -> String;
    async fn reload(&mut self);
    fn capabilities(&self) -> Capabilities;
    fn supported_priorities(&self) -> Vec<Priority> {
        Priority::values()
    }
}
