// SPDX-License-Identifier: MIT

pub mod client;
mod project;
mod task;

use std::{cmp::Ordering, error::Error, fmt::Debug};
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
    task::{Priority, State, Task as TaskTrait},
    task_patch::{DatePatchItem, PatchError, TaskPatch},
};

use async_trait::async_trait;

use crate::config::Config;

pub const PROVIDER_NAME: &str = "Todoist";

pub struct Provider {
    cfg: Config,
    c: client::Client,
    projects: Vec<project::Project>,
    tasks: Vec<task::Task>,
    last_filter: Option<filter::Filter>,
    last_project: Option<Box<dyn ProjectTrait>>,
}

impl Provider {
    pub fn new(cfg: Config, api_key: &str) -> Self {
        Self {
            cfg,
            c: client::Client::new(api_key),
            projects: Vec::new(),
            tasks: Vec::new(),
            last_filter: None,
            last_project: None,
        }
    }

    async fn load_projects(&mut self) -> Result<(), Box<dyn Error>> {
        if self.projects.is_empty() {
            self.projects = self.c.projects().await?;
            for p in &mut self.projects {
                p.provider = Some(self.cfg.name());
            }
        }
        Ok(())
    }

    pub async fn project_by_id(&mut self, id: &str) -> Result<project::Project, Box<dyn Error>> {
        self.load_projects().await?;
        let project = self.projects.iter().find(|p| p.id() == id);
        if let Some(p) = project {
            return Ok(p.clone());
        }
        Ok(project::Project {
            id: id.to_string(),
            ..project::Project::default()
        })
    }
}

impl Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider name={}", ProviderTrait::name(self))
    }
}

#[async_trait]
impl ProjectProviderTrait for Provider {
    async fn list(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        self.load_projects().await?;
        let mut result: Vec<Box<dyn ProjectTrait>> = Vec::new();
        for p in &self.projects {
            result.push(Box::new(p.clone()));
        }

        Ok(result)
    }
}

#[async_trait]
impl TaskProviderTrait for Provider {
    #[tracing::instrument(level = "info", target = "todoist_tasks")]
    async fn list(
        &mut self,
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        let mut should_clear = false;
        if let Some(last_filter) = self.last_filter.as_mut() {
            should_clear = last_filter != f;
        }

        match &project {
            Some(p) => {
                if let Some(pp) = self.last_project.as_mut() {
                    should_clear |= p.id().cmp(&pp.id()) != Ordering::Equal;
                } else {
                    should_clear = true
                }
            }
            None => {
                if self.last_project.is_some() {
                    should_clear = true
                }
            }
        }

        if should_clear {
            self.tasks.clear();
        }

        if self.tasks.is_empty() {
            if f.states.contains(&filter::FilterState::Uncompleted) {
                match self.c.tasks_by_filter(&project, f).await {
                    Ok(mut t) => self.tasks.append(&mut t),
                    Err(e) => {
                        tracing::error!(error=?e,  "Get tasks by filter");
                        return Err(e.into());
                    }
                }
            }

            if f.states.contains(&filter::FilterState::Completed) {
                match self.c.completed_tasks(&project.as_ref().map(|p| p.id()), f).await {
                    Ok(mut tasks) => self.tasks.append(&mut tasks),
                    Err(e) => {
                        tracing::error!(error=?e,  "Get completed tasks");
                        return Err(e.into());
                    }
                }
            }
            self.last_project = project;
        }

        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();

        for t in &mut self.tasks.to_vec() {
            t.project = Some(self.project_by_id(t.project_id.as_str()).await?);
            t.provider = Some(self.name());
            result.push(Box::new(t.clone()));
        }

        self.last_filter = Some(f.clone());

        Ok(result)
    }

    async fn create(&mut self, project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        let mut due_custom_dt = String::new();

        let name = tp.name.value().unwrap();
        let description = tp.description.value();
        let r = client::CreateTaskRequest {
            content: name.as_str(),
            description: description.as_deref(),
            project_id: Some(project_id),
            due_string: tp.due.value().map(|due| match due {
                DatePatchItem::NoDate => "no date",
                DatePatchItem::Today => "today",
                DatePatchItem::Tomorrow => "tomorrow",
                DatePatchItem::ThisWeekend => "weekend",
                DatePatchItem::NextWeek => "next week",
                DatePatchItem::Custom(dt) => {
                    due_custom_dt = dt.format("%Y-%m-%d").to_string();
                    &due_custom_dt
                }
            }),
            priority: tp.priority.value().map(|p| task::priority_to_int(&p)),
        };
        self.c.create_task(&r).await.map_err(|e| e.into())
    }

    async fn update(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut errors = Vec::new();

        for p in patches {
            let task = p.task.as_ref().unwrap();

            if let Some(state) = &p.state.value() {
                match state {
                    State::Completed => match self.c.close_task(task.id().as_str()).await {
                        Ok(_) => self.tasks.clear(),
                        Err(e) => errors.push(PatchError {
                            task: task.clone_boxed(),
                            error: e.to_string(),
                        }),
                    },
                    State::InProgress | State::Unknown(_) => errors.push(PatchError {
                        task: task.clone_boxed(),
                        error: format!("The state {state} is unsupported"),
                    }),
                    State::Uncompleted => match self.c.reopen_task(task.id().as_str()).await {
                        Ok(_) => self.tasks.clear(),
                        Err(e) => errors.push(PatchError {
                            task: task.clone_boxed(),
                            error: e.to_string(),
                        }),
                    },
                }
            }

            if p.due.is_set() || p.priority.is_set() || p.description.is_set() || p.name.is_set() {
                let mut due_custom_dt = String::new();

                let name = p.name.value();
                let description = p.description.value();
                let r = client::UpdateTaskRequest {
                    content: name.as_deref(),
                    description: description.as_deref(),
                    due_string: p.due.value().map(|due| match due {
                        DatePatchItem::NoDate => "no date",
                        DatePatchItem::Today => "today",
                        DatePatchItem::Tomorrow => "tomorrow",
                        DatePatchItem::ThisWeekend => "weekend",
                        DatePatchItem::NextWeek => "next week",
                        DatePatchItem::Custom(dt) => {
                            due_custom_dt = dt.format("%Y-%m-%d").to_string();
                            &due_custom_dt
                        }
                    }),
                    priority: p.priority.value().map(|p| task::priority_to_int(&p)),
                };
                match self.c.update_task(task.id().as_str(), &r).await {
                    Ok(_) => self.tasks.clear(),
                    Err(e) => errors.push(PatchError {
                        task: task.clone_boxed(),
                        error: e.to_string(),
                    }),
                }
            }
        }

        errors
    }

    async fn delete(&mut self, t: &dyn TaskTrait) -> Result<(), StringError> {
        self.c.delete_task(t.id().as_str()).await.map_err(|e| e.into())
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
        self.projects.clear();
        self.tasks.clear();
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: true }
    }

    fn supported_priorities(&self) -> Vec<Priority> {
        task::SUPPORTED_PRIORITIES.into()
    }
}
