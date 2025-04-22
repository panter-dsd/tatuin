pub mod client;
mod project;
mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::task::Provider as ProviderTrait;
use crate::task::Task as TaskTrait;

use async_trait::async_trait;

const PROVIDER_NAME: &str = "Todoist";

pub struct Provider {
    c: client::Client,
}

impl Provider {
    pub fn new(api_key: &str) -> Self {
        Self {
            c: client::Client::new(api_key),
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
        &self,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn std::error::Error>> {
        let tasks = self.c.tasks_by_filter(&None, f).await?;
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for t in tasks {
            result.push(Box::new(t));
        }
        Ok(result)
    }

    async fn projects(&self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn std::error::Error>> {
        let projects = self.c.projects().await?;
        let mut result: Vec<Box<dyn ProjectTrait>> = Vec::new();
        for p in projects {
            result.push(Box::new(p));
        }

        Ok(result)
    }
}
