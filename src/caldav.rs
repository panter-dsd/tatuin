// SPDX-License-Identifier: MIT

mod client;
mod fake_project;
use async_trait::async_trait;
use ratatui::style::Color;

use crate::{
    filter, folders,
    ical::Task,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProviderTrait, StringError},
    task::{Priority, Task as TaskTrait},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
};
use client::{Client, Config};

pub const PROVIDER_NAME: &str = "CalDav";

pub struct Provider {
    name: String,
    color: Color,

    c: Client,
    tasks: Vec<Task>,
}

impl Provider {
    pub fn new(name: &str, url: &str, login: &str, password: &str, color: &Color) -> Self {
        let mut s = Self {
            name: name.to_string(),
            color: *color,
            c: Client::new(Config {
                url: url.to_string(),
                login: login.to_string(),
                password: password.to_string(),
            }),
            tasks: Vec::new(),
        };

        if let Ok(f) = folders::provider_cache_folder(&s) {
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

    #[tracing::instrument(level = "info", target = "caldav_tasks")]
    async fn tasks(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        if self.tasks.is_empty() {
            self.c.download().await?;
            self.tasks = self
                .c
                .parse_calendars()
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
        Ok(vec![Box::new(fake_project::Project::default())])
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
        Capabilities { create_task: true }
    }

    async fn create_task(&mut self, _project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let t = Task {
            provider: PROVIDER_NAME.to_string(),
            properties: Vec::new(),
            name: tp.name.clone().unwrap(),
            description: tp.description.clone(),
            due: tp.due.unwrap_or(DuePatchItem::NoDate).into(),
            priority: tp.priority.unwrap_or(Priority::Normal).into(),
            ..Task::default()
        };
        self.c.create_or_update(&t).await.map_err(|e| {
            tracing::error!(target:"caldav_provider",  error=?e, "Create a task");
            StringError::new(e.to_string().as_str())
        })
    }
}
