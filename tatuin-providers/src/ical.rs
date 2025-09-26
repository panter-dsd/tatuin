// SPDX-License-Identifier: MIT

mod client;
mod priority;
mod task;
use std::error::Error;

use async_trait::async_trait;

use client::Client;
pub use client::parse_calendar;
pub use task::{Task, TaskType, property_to_str};
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
    task::Task as TaskTrait,
    task_patch::{PatchError, TaskPatch},
};

use crate::config::Config;

pub const PROVIDER_NAME: &str = "iCal";

pub struct Provider {
    cfg: Config,

    c: Client,
    tasks: Vec<Task>,
}

impl Provider {
    pub fn new(cfg: Config, url: &str) -> Result<Self, Box<dyn Error>> {
        let mut c = Client::new(url);
        c.set_cache_folder(&cfg.cache_path()?);
        Ok(Self {
            cfg,
            c,
            tasks: Vec::new(),
        })
    }
}

impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider name={}", ProviderTrait::name(self))
    }
}

#[async_trait]
impl ProjectProviderTrait for Provider {
    async fn list(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        Err(StringError::new("not implemented"))
    }
}

#[async_trait]
impl TaskProviderTrait for Provider {
    #[tracing::instrument(level = "info", target = "ical_tasks")]
    async fn list(
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
                    task.set_provider(self.cfg.name().as_str());
                    task
                })
                .collect();
        }

        return Ok(self.tasks.iter().map(|t| t.clone_boxed()).collect());
    }

    async fn create(&mut self, _project_id: &str, _tp: &TaskPatch) -> Result<(), StringError> {
        panic!("Not implemented")
    }

    async fn update(&mut self, _patches: &[TaskPatch]) -> Vec<PatchError> {
        panic!("Not implemented")
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.cfg.name()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn reload(&mut self) {
        self.tasks.clear();
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: false }
    }
}
