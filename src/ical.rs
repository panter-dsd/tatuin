// SPDX-License-Identifier: MIT

mod client;
mod priority;
mod task;
use async_trait::async_trait;
use ratatui::style::Color;

use crate::APP_NAME;
use client::Client;
pub use client::parse_calendar;
pub use task::{Task, TaskType, property_to_str};
use tatuin_core::{
    filter, folders,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProviderTrait, StringError},
    task::Task as TaskTrait,
    task_patch::{PatchError, TaskPatch},
};

pub const PROVIDER_NAME: &str = "iCal";

pub struct Provider {
    name: String,
    color: Color,

    c: Client,
    tasks: Vec<Task>,
}

impl Provider {
    pub fn new(name: &str, url: &str, color: &Color) -> Self {
        let mut s = Self {
            name: name.to_string(),
            color: *color,
            c: Client::new(url),
            tasks: Vec::new(),
        };

        if let Ok(f) = folders::provider_cache_folder(APP_NAME, &s) {
            s.c.set_cache_folder(&f);
        }
        s
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

    #[tracing::instrument(level = "info", target = "ical_tasks")]
    async fn tasks(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        if self.tasks.is_empty() {
            self.c.download_calendar().await?;
            self.tasks = self
                .c
                .parse_calendar()
                .await?
                .iter()
                .filter(|t| f.accept(*t))
                .map(|t| {
                    let mut task = t.clone();
                    task.set_provider(self.name.as_str());
                    task
                })
                .collect();
        }

        return Ok(self.tasks.iter().map(|t| t.clone_boxed()).collect());
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        Err(StringError::new("not implemented"))
    }

    async fn patch_tasks(&mut self, _patches: &[TaskPatch]) -> Vec<PatchError> {
        panic!("Not implemented")
    }

    async fn reload(&mut self) {
        self.tasks.clear();
    }

    fn color(&self) -> Color {
        self.color
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: false }
    }

    async fn create_task(&mut self, _project_id: &str, _tp: &TaskPatch) -> Result<(), StringError> {
        panic!("Not implemented")
    }
}
