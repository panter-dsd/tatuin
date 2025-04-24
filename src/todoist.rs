pub mod client;
mod project;
mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::task::Task as TaskTrait;

use async_trait::async_trait;

const PROVIDER_NAME: &str = "Todoist";

pub struct Provider {
    c: client::Client,
    projects: Vec<project::Project>,
    tasks: Vec<task::Task>,
    last_filter: Option<filter::Filter>,
}

impl Provider {
    pub fn new(api_key: &str) -> Self {
        Self {
            c: client::Client::new(api_key),
            projects: Vec::new(),
            tasks: Vec::new(),
            last_filter: None,
        }
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &mut self,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn std::error::Error>> {
        if let Some(last_filter) = self.last_filter.as_mut() {
            if last_filter != f {
                self.tasks.clear();
            }
        }

        if self.tasks.is_empty() {
            self.tasks = self.c.tasks(&None, f).await?;
        }
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for t in &self.tasks {
            result.push(Box::new(t.clone()));
        }

        self.last_filter = Some(f.clone());

        Ok(result)
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn std::error::Error>> {
        if self.projects.is_empty() {
            self.projects = self.c.projects().await?;
        }
        let mut result: Vec<Box<dyn ProjectTrait>> = Vec::new();
        for p in &self.projects {
            result.push(Box::new(p.clone()));
        }

        Ok(result)
    }
}
