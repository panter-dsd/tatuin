// SPDX-License-Identifier: MIT

mod client;
mod md_file;
mod project;
mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::provider::{PatchError, Provider as ProviderTrait, TaskPatch};
use crate::task::Task as TaskTrait;
use async_trait::async_trait;
use ratatui::style::Color;
use std::error::Error;

pub const PROVIDER_NAME: &str = "Obsidian";

pub struct Provider {
    name: String,
    c: client::Client,
    color: Color,
}

impl Provider {
    pub fn new(name: &str, path: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            c: client::Client::new(path),
            color: *color,
        }
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
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn Error>> {
        let tasks = self.c.tasks(f).await?;
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for mut t in tasks {
            t.set_provider(self.name());
            result.push(Box::new(t));
        }
        Ok(result)
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>> {
        Ok(Vec::new())
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut client_patches = Vec::new();
        let mut errors = Vec::new();
        for p in patches.iter() {
            let st = match &p.state {
                Some(s) => s,
                None => {
                    errors.push(PatchError {
                        task: p.task.clone_boxed(),
                        error: "The only state changing is implemented".to_string(),
                    });
                    continue;
                }
            };
            match p.task.as_any().downcast_ref::<task::Task>() {
                Some(t) => client_patches.push(client::TaskPatch {
                    task: t,
                    state: st.clone().into(),
                }),
                None => panic!("Wrong casting!"),
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
}
