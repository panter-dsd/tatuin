// SPDX-License-Identifier: MIT

pub mod client;
mod project;
mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::provider::{PatchError, Provider as ProviderTrait, TaskPatch};
use crate::task::{State, Task as TaskTrait};
use ratatui::style::Color;
use std::cmp::Ordering;
use std::error::Error;

use async_trait::async_trait;

pub const PROVIDER_NAME: &str = "Todoist";

pub struct Provider {
    name: String,
    c: client::Client,
    color: Color,
    projects: Vec<project::Project>,
    tasks: Vec<task::Task>,
    last_filter: Option<filter::Filter>,
    last_project: Option<Box<dyn ProjectTrait>>,
}

impl Provider {
    pub fn new(name: &str, api_key: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            c: client::Client::new(api_key),
            color: *color,
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
                p.provider = Some(self.name.to_string());
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
        project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn Error>> {
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
                self.tasks.append(&mut self.c.tasks_by_filter(&project, f).await?);
            }

            if f.states.contains(&filter::FilterState::Completed) {
                self.tasks
                    .append(&mut self.c.completed_tasks(&project.as_ref().map(|p| p.id()), f).await?);
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

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>> {
        self.load_projects().await?;
        let mut result: Vec<Box<dyn ProjectTrait>> = Vec::new();
        for p in &self.projects {
            result.push(Box::new(p.clone()));
        }

        Ok(result)
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut errors = Vec::new();

        for p in patches {
            if let Some(state) = &p.state {
                match state {
                    State::Completed => match self.c.close_task(p.task.id().as_str()).await {
                        Ok(_) => self.tasks.clear(),
                        Err(e) => errors.push(PatchError {
                            task: p.task.clone_boxed(),
                            error: e.to_string(),
                        }),
                    },
                    State::InProgress | State::Unknown(_) => errors.push(PatchError {
                        task: p.task.clone_boxed(),
                        error: format!("The state {state} is unsupported"),
                    }),
                    State::Uncompleted => match self.c.reopen_task(p.task.id().as_str()).await {
                        Ok(_) => self.tasks.clear(),
                        Err(e) => errors.push(PatchError {
                            task: p.task.clone_boxed(),
                            error: e.to_string(),
                        }),
                    },
                }
            }
        }

        errors
    }

    async fn reload(&mut self) {
        self.projects.clear();
        self.tasks.clear();
    }

    fn color(&self) -> Color {
        self.color
    }
}
