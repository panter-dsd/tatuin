// SPDX-License-Identifier: MIT

mod client;
mod project;
mod task;

use std::error::Error;

use async_trait::async_trait;
use client::Client;
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProviderTrait},
    task::{Priority, Task as TaskTrait},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
};

use crate::config::Config;

pub const PROVIDER_NAME: &str = "Tatuin";

fn parse_uuid(s: &str) -> Result<uuid::Uuid, Box<dyn Error>> {
    uuid::Uuid::parse_str(s).map_err(|e| {
        tracing::error!(error=?e, string=s, "Parse uuid from string");
        StringError::new(e.to_string().as_str()).into()
    })
}

pub struct Provider {
    cfg: Config,
    c: Client,
}

impl Provider {
    pub fn new(cfg: Config) -> Result<Self, Box<dyn Error>> {
        let c = Client::new(&cfg.cache_path(PROVIDER_NAME)?)?;
        Ok(Self { cfg, c })
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
        self.cfg.name()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        let project_id = if let Some(p) = project {
            Some(parse_uuid(p.id().as_str())?)
        } else {
            None
        };

        let projects = self.c.projects().await?;

        let provider_name = self.name();
        Ok(self
            .c
            .tasks(project_id, f)
            .await?
            .iter_mut()
            .map(|t| {
                t.set_provider(&provider_name);
                if let Some(p) = projects.iter().find(|p| p.id == t.project_id) {
                    t.set_project(p.clone());
                }
                t.clone_boxed()
            })
            .collect::<Vec<Box<dyn TaskTrait>>>())
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        self.c
            .projects()
            .await
            .map(|v| v.iter().map(|p| p.clone_boxed()).collect())
            .map_err(|e| {
                tracing::error!(error=?e, "Get projects from database");
                e.into()
            })
    }

    async fn patch_tasks(&mut self, _patches: &[TaskPatch]) -> Vec<PatchError> {
        todo!("Implement me")
    }

    async fn reload(&mut self) {
        // do nothing for now
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: true }
    }

    async fn create_task(&mut self, project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let mut t = task::Task::default();
        t.id = uuid::Uuid::new_v4();
        t.name = tp.name.clone().unwrap();
        t.description = tp.description.clone();
        t.due = tp.due.unwrap_or(DuePatchItem::NoDate).into();
        t.priority = tp.priority.unwrap_or(Priority::Normal);
        t.project_id = parse_uuid(project_id)?;
        self.c.create_task(t).await.map_err(|e| {
            tracing::error!(error=?e, "Insert task into database");
            e.into()
        })
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use tatuin_core::{filter::Filter, project::Project, provider::ProviderTrait, task_patch::TaskPatch};

    use crate::{config::Config, tatuin::project::inbox_project};

    use super::Provider;

    fn config(cache_path: PathBuf) -> Config {
        let mut cfg = Config::new("test_app", "test_name");
        cfg.cache_path = cache_path;
        cfg
    }

    #[tokio::test]
    async fn get_projects_on_empty_provider() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p = Provider::new(config(temp_dir.path().to_path_buf()));
        assert!(p.is_ok());

        let p: &mut dyn ProviderTrait = &mut p.unwrap();

        let projects = p.projects().await;
        assert!(projects.is_ok());

        let projects = projects.unwrap();
        assert_eq!(projects.len(), 1);

        let project = &projects[0];
        let inbox = inbox_project();
        assert_eq!(project.name(), inbox.name());
        assert_eq!(project.description(), inbox.description());
        assert_eq!(project.provider(), inbox.provider());
        assert_eq!(project.parent_id(), inbox.parent_id());
        assert_eq!(project.is_inbox(), inbox.is_inbox());
        assert_eq!(project.is_favorite(), inbox.is_favorite());
    }

    #[tokio::test]
    async fn get_tasks_on_empty_provider() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p = Provider::new(config(temp_dir.path().to_path_buf()));
        assert!(p.is_ok());

        let p: &mut dyn ProviderTrait = &mut p.unwrap();

        let tasks = p.tasks(None, &Filter::full_filter()).await;
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn create_tasks() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p: &mut dyn ProviderTrait = &mut Provider::new(config(temp_dir.path().to_path_buf())).unwrap();

        let project = &p.projects().await.unwrap()[0];

        let tasks = p.tasks(None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), 0);

        let mut patches = Vec::new();
        for i in 0..100 {
            let tp = TaskPatch {
                task: None,
                name: Some(format!("Name {i}")),
                description: Some(format!("Description {i}")),
                due: Some(tatuin_core::task_patch::DuePatchItem::Today),
                priority: Some(tatuin_core::task::Priority::Low),
                state: None,
            };
            let r = p.create_task(project.id().as_str(), &tp).await;
            patches.push(tp);
            assert!(r.is_ok())
        }

        let tasks = p.tasks(None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), patches.len());
    }
}
