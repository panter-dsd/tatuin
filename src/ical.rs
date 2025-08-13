mod client;
use async_trait::async_trait;
use ratatui::style::Color;

use crate::{
    filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProviderTrait, StringError},
    task::Task as TaskTrait,
    task_patch::{PatchError, TaskPatch},
};
use client::Client;

pub const PROVIDER_NAME: &str = "iCal";

pub struct Provider {
    name: String,
    color: Color,

    c: Client,
}

impl Provider {
    pub fn new(name: &str, url: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            color: *color,
            c: Client::new(url),
        }
    }
}

impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider name={}", ProviderTrait::name(self))
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    #[tracing::instrument(level = "info", target = "todoist_tasks")]
    async fn tasks(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        todo!("Not implemented")
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        todo!("Not implemented")
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        todo!("Not implemented")
    }

    async fn reload(&mut self) {}

    fn color(&self) -> Color {
        self.color
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: false }
    }

    async fn create_task(&mut self, project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        todo!("Not implemented")
    }
}
