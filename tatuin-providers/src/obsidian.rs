// SPDX-License-Identifier: MIT

mod client;
mod indent;
mod md_file;
mod patch;
mod project;
mod rest;
mod task;

use async_trait::async_trait;
use md_file::task_to_string;
use task::Description;
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
    task::{Priority, Task as TaskTrait},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
};

use crate::config::Config;

pub const PROVIDER_NAME: &str = "Obsidian";

pub struct Provider {
    cfg: Config,
    c: client::Client,
    rest: rest::Client,
}

impl Provider {
    pub fn new(cfg: Config, path: &str) -> Self {
        Self {
            cfg,
            c: client::Client::new(path),
            rest: rest::Client::new(path),
        }
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
        Ok(vec![Box::new(project::Project::new(
            self.cfg.name().as_str(),
            self.c.root_path().as_str(),
            format!("{}/daily.md", self.c.root_path()).as_str(),
        ))])
    }
}

#[async_trait]
impl TaskProviderTrait for Provider {
    async fn list(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        let tasks = self.c.tasks(f).await?;
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for mut t in tasks {
            t.set_provider(self.name());
            result.push(Box::new(t));
        }
        Ok(result)
    }

    async fn create(&mut self, _project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let t = task::Task {
            text: tp.name.value().unwrap(),
            description: tp.description.value().map(|s| Description::from_str(s.as_str())),
            state: task::State::Uncompleted,
            due: tp.due.value().unwrap_or(DuePatchItem::NoDate).into(),
            priority: tp.priority.value().unwrap_or(Priority::Normal),
            ..task::Task::default()
        };
        self.rest.add_text_to_daily_note(task_to_string(&t, "").as_str()).await
    }

    async fn update(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut client_patches = Vec::new();
        let mut errors = Vec::new();
        for p in patches.iter() {
            let task = p.task.as_ref().unwrap();

            match task.as_any().downcast_ref::<task::Task>() {
                Some(t) => client_patches.push(patch_to_internal(t, p)),
                None => panic!(
                    "Wrong casting the task id=`{}` name=`{}` to obsidian!",
                    task.id(),
                    task.text(),
                ),
            };
        }

        for e in self.c.patch_tasks(&client_patches).await {
            errors.push(PatchError {
                task: e.task.clone_boxed(),
                error: e.error,
            })
        }

        errors
    }

    async fn delete(&mut self, t: &dyn TaskTrait) -> Result<(), StringError> {
        let t = t.as_any().downcast_ref::<task::Task>().expect("Wrong casting");
        self.c.delete_task(t).await.map_err(|e| {
            tracing::error!(error=?e, name=t.text(), id=t.id(), "Delete the task");
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
        Capabilities {
            create_task: self.rest.is_available(),
        }
    }
}

fn patch_to_internal<'a>(t: &'a task::Task, tp: &TaskPatch) -> patch::TaskPatch<'a> {
    patch::TaskPatch {
        task: t,
        name: tp.name.value(),
        description: tp.description.value(),
        state: tp.state.value().map(|s| s.into()),
        due: match tp.due.value() {
            Some(due) => due.into(),
            None => None,
        },
        priority: tp.priority.value(),
    }
}
