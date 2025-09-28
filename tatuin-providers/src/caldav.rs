// SPDX-License-Identifier: MIT

mod client;
mod fake_project;

use std::error::Error;

use async_trait::async_trait;

use super::ical::Task;
use crate::config::Config as ProviderConfig;
use client::{Client, Config};
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
    task::{Priority, State, Task as TaskTrait},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
};

pub const PROVIDER_NAME: &str = "CalDav";

pub struct Provider {
    cfg: ProviderConfig,

    c: Client,
    tasks: Vec<Task>,
}

impl Provider {
    pub fn new(cfg: ProviderConfig, url: &str, login: &str, password: &str) -> Result<Self, Box<dyn Error>> {
        let mut c = Client::new(Config {
            url: url.to_string(),
            login: login.to_string(),
            password: password.to_string(),
        });
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
        Ok(vec![Box::new(fake_project::Project::default())])
    }
}

#[async_trait]
impl TaskProviderTrait for Provider {
    #[tracing::instrument(level = "info", target = "caldav_tasks")]
    async fn list(
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
                    task.set_provider(self.cfg.name().as_str());
                    task
                })
                .collect();
        }

        return Ok(self.tasks.iter().map(|t| t.clone_boxed()).collect());
    }

    async fn create(&mut self, _project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let t = Task {
            provider: PROVIDER_NAME.to_string(),
            properties: Vec::new(),
            name: tp.name.value().unwrap(),
            description: tp.description.value(),
            due: tp.due.value().unwrap_or(DuePatchItem::NoDate).into(),
            priority: tp.priority.value().unwrap_or(Priority::Normal).into(),
            ..Task::default()
        };
        self.c.create_or_update(&t).await.map_err(|e| {
            tracing::error!(target:"caldav_provider",  error=?e, "Create a task");
            StringError::new(e.to_string().as_str())
        })
    }

    async fn update(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut errors = Vec::new();
        for p in patches.iter() {
            let task = p.task.as_ref().unwrap();

            match task.as_any().downcast_ref::<Task>() {
                Some(t) => {
                    let mut t = t.clone();
                    t.name = p.name.value().unwrap_or(t.name);
                    if p.description.is_set() {
                        t.description = p.description.value();
                    }
                    if let Some(due) = p.due.value() {
                        t.due = due.into();
                    }
                    if let Some(p) = p.priority.value() {
                        t.priority = p.into();
                    }
                    if let Some(s) = p.state.value() {
                        t.status = s.into();
                        if s == State::Completed {
                            t.completed = Some(chrono::Utc::now());
                        } else {
                            t.completed = None;
                        }
                    }
                    let r = self.c.create_or_update(&t).await.map_err(|e| {
                        tracing::error!(target:"caldav_provider",  error=?e, "Patch the task");
                        PatchError {
                            task: t.clone_boxed(),
                            error: e.to_string(),
                        }
                    });
                    if let Err(e) = r {
                        errors.push(e);
                    }
                }
                None => panic!(
                    "Wrong casting the task id=`{}` name=`{}` to obsidian!",
                    task.id(),
                    task.text(),
                ),
            };
        }

        errors
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
        Capabilities { create_task: true }
    }
}
