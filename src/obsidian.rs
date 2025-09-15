// SPDX-License-Identifier: MIT

mod client;
mod md_file;
mod patch;
mod project;
mod rest;
mod task;

use async_trait::async_trait;
use md_file::task_to_string;
use ratatui::style::Color;
use tatuin_core::{
    filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProviderTrait, StringError},
    task::{Priority, Task as TaskTrait},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
};

pub const PROVIDER_NAME: &str = "Obsidian";

pub struct Provider {
    name: String,
    c: client::Client,
    rest: rest::Client,
    color: Color,
}

impl Provider {
    pub fn new(name: &str, path: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            c: client::Client::new(path),
            rest: rest::Client::new(path),
            color: *color,
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

    async fn tasks(
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

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        Ok(vec![Box::new(project::Project::new(
            self.name.as_str(),
            self.c.root_path().as_str(),
            format!("{}/daily.md", self.c.root_path()).as_str(),
        ))])
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
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

    async fn reload(&mut self) {
        // do nothing for now
    }

    fn color(&self) -> Color {
        self.color
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            create_task: self.rest.is_available(),
        }
    }

    async fn create_task(&mut self, _project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let t = task::Task {
            text: tp.name.clone().unwrap(),
            state: task::State::Uncompleted,
            due: tp.due.unwrap_or(DuePatchItem::NoDate).into(),
            priority: tp.priority.unwrap_or(Priority::Normal),
            ..task::Task::default()
        };
        self.rest.add_text_to_daily_note(task_to_string(&t).as_str()).await
    }
}

fn patch_to_internal<'a>(t: &'a task::Task, tp: &TaskPatch) -> patch::TaskPatch<'a> {
    patch::TaskPatch {
        task: t,
        name: tp.name.clone(),
        state: tp.state.map(|s| s.into()),
        due: match tp.due {
            Some(due) => due.into(),
            None => None,
        },
        priority: tp.priority,
    }
}
