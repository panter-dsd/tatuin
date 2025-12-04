// SPDX-License-Identifier: MIT

mod client;
mod project;
mod task;

use std::error::Error;

use async_trait::async_trait;
use chrono::Utc;
use client::Client;
use task::Task;
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
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
        let c = Client::new(&cfg.cache_path()?);
        Ok(Self { cfg, c })
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
        self.c
            .projects(self.cfg.name().as_str())
            .await
            .map(|v| v.iter().map(|p| p.clone_boxed()).collect())
            .map_err(|e| {
                tracing::error!(error=?e, "Get projects from database");
                e.into()
            })
    }
}

#[async_trait]
impl TaskProviderTrait for Provider {
    async fn list(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        let project_id = if let Some(p) = project {
            Some(parse_uuid(p.id().as_str())?)
        } else {
            None
        };

        let projects = self.c.projects(self.cfg.name().as_str()).await?;

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

    async fn create(&mut self, project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let mut t = task::Task::default();
        t.id = uuid::Uuid::new_v4();
        t.name = tp.name.value().unwrap();
        t.description = tp.description.value();
        t.due = tp.due.value().unwrap_or(DuePatchItem::NoDate).into();
        t.priority = tp.priority.value().unwrap_or(Priority::Normal);
        t.project_id = parse_uuid(project_id)?;
        t.created_at = Utc::now();
        t.updated_at = Utc::now();
        self.c.create_task(t).await.map_err(|e| {
            tracing::error!(error=?e, "Insert task into database");
            e.into()
        })
    }

    async fn update(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let tasks = patches.iter().map(task_patch_to_task).collect::<Vec<Task>>();
        self.c.patch_tasks(&tasks).await
    }

    async fn delete(&mut self, t: &dyn TaskTrait) -> Result<(), StringError> {
        let t = t.as_any().downcast_ref::<Task>().expect("Wrong casting");
        self.c.delete_task(t).await.map_err(|e| {
            tracing::error!(error=?e, "Delete the task from database");
            e.into()
        })
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
        // do nothing for now
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: true }
    }
}

fn task_patch_to_task(tp: &TaskPatch) -> Task {
    let mut t = tp
        .task
        .as_ref()
        .expect("Task in patch should be exist")
        .as_any()
        .downcast_ref::<Task>()
        .expect("The task should have right type")
        .clone();

    if let Some(n) = &tp.name.value() {
        t.name = n.clone();
    }

    if tp.description.is_set() {
        t.description = tp.description.value();
    }

    if let Some(p) = &tp.priority.value() {
        t.priority = *p;
    }

    if let Some(s) = &tp.state.value() {
        t.state = *s;
    }

    if tp.due.is_set() {
        t.due = match tp.due.value() {
            Some(d) => d.into(),
            None => None,
        }
    }

    t
}

#[cfg(test)]
mod test {
    use std::{error::Error, path::PathBuf};

    use super::task;
    use tatuin_core::{
        filter::Filter,
        project::Project,
        provider::{ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
        task::{Priority, State},
        task_patch::{DuePatchItem, TaskPatch, ValuePatch},
    };

    use crate::{config::Config, tatuin::project::inbox_project};

    use super::Provider;

    fn config(cache_path: PathBuf) -> Config {
        let mut cfg = Config::new("test_app", "test_name");
        cfg.cache_path = cache_path;
        cfg
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn get_projects_on_empty_provider() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p = Provider::new(config(temp_dir.path().to_path_buf()));
        assert!(p.is_ok());

        let p: &mut dyn ProjectProviderTrait = &mut p.unwrap();

        let projects = p.list().await;
        assert!(projects.is_ok());

        let projects = projects.unwrap();
        assert_eq!(projects.len(), 1);

        let project = &projects[0];
        let inbox = inbox_project("test_name");
        assert_eq!(project.name(), inbox.name());
        assert_eq!(project.description(), inbox.description());
        assert_eq!(project.provider(), inbox.provider());
        assert_eq!(project.parent_id(), inbox.parent_id());
        assert_eq!(project.is_inbox(), inbox.is_inbox());
        assert_eq!(project.is_favorite(), inbox.is_favorite());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn get_tasks_on_empty_provider() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p = Provider::new(config(temp_dir.path().to_path_buf()));
        assert!(p.is_ok());

        let p: &mut dyn TaskProviderTrait = &mut p.unwrap();

        let tasks = p.list(None, &Filter::full_filter()).await;
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    fn generate_task_patch(i: u16) -> TaskPatch {
        TaskPatch {
            task: None,
            name: ValuePatch::Value(format!("Name {i}")),
            description: if i.is_multiple_of(2) {
                ValuePatch::Value(format!("Description {i}"))
            } else {
                ValuePatch::NotSet
            },
            due: if i.is_multiple_of(3) {
                ValuePatch::Value(DuePatchItem::Today)
            } else {
                ValuePatch::NotSet
            },
            priority: if i.is_multiple_of(5) {
                ValuePatch::Value(Priority::Low)
            } else {
                ValuePatch::NotSet
            },
            state: if i.is_multiple_of(2) {
                ValuePatch::Value(State::Completed)
            } else {
                ValuePatch::NotSet
            },
        }
    }

    async fn generate_items(
        p: &mut dyn ProviderTrait,
        count: u16,
        project_id: &str,
    ) -> Result<Vec<TaskPatch>, Box<dyn Error>> {
        let mut patches = Vec::new();
        for i in 0..count {
            let tp = generate_task_patch(i);
            p.create(project_id, &tp).await?;
            patches.push(tp);
        }

        Ok(patches)
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn create_tasks() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p: &mut dyn ProviderTrait = &mut Provider::new(config(temp_dir.path().to_path_buf())).unwrap();

        let project = &ProjectProviderTrait::list(p).await.unwrap()[0];

        let tasks = TaskProviderTrait::list(p, None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), 0);

        let patches = generate_items(p, 10, project.id().as_str()).await;
        assert!(patches.is_ok());
        let patches = patches.unwrap();

        let tasks = TaskProviderTrait::list(p, None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), patches.len());

        for t in tasks {
            let found = patches.iter().any(|tp| {
                *tp.name.value().unwrap() == t.name().raw()
                    && tp.description.value() == t.description().map(|d| d.raw())
                    && tp.due.value() == t.due().map(|d| d.into())
                    && tp.priority.value().unwrap_or(Priority::Normal) == t.priority()
                    && t.state() == State::Uncompleted
            });
            assert!(
                found,
                "Task {:?} was not found",
                t.as_any().downcast_ref::<task::Task>()
            );
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn mark_task_as_completed() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p: &mut dyn ProviderTrait = &mut Provider::new(config(temp_dir.path().to_path_buf())).unwrap();

        let project = &ProjectProviderTrait::list(p).await.unwrap()[0];

        let tasks = TaskProviderTrait::list(p, None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), 0);

        let patches = generate_items(p, 10, project.id().as_str()).await;
        assert!(patches.is_ok());
        let patches = patches.unwrap();

        let tasks = TaskProviderTrait::list(p, None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), patches.len());

        let complete_patches = tasks
            .iter()
            .map(|t| TaskPatch {
                task: Some(t.clone_boxed()),
                name: ValuePatch::NotSet,
                description: ValuePatch::NotSet,
                due: ValuePatch::NotSet,
                priority: ValuePatch::NotSet,
                state: ValuePatch::Value(State::Completed),
            })
            .collect::<Vec<TaskPatch>>();
        let patch_errors = p.update(&complete_patches).await;
        assert!(patch_errors.is_empty());

        let tasks = TaskProviderTrait::list(p, None, &Filter::full_filter()).await.unwrap();
        assert_eq!(tasks.len(), patches.len());

        for t in tasks {
            let found = patches.iter().any(|tp| {
                *tp.name.value().unwrap() == t.name().raw()
                    && tp.description.value() == t.description().map(|d| d.raw())
                    && tp.due.value() == t.due().map(|d| d.into())
                    && tp.priority.value().unwrap_or(Priority::Normal) == t.priority()
                    && t.state() == State::Completed
            });
            assert!(
                found,
                "Task {:?} was not found",
                t.as_any().downcast_ref::<task::Task>()
            );
        }
    }
}
