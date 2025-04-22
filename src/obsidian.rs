pub mod client;
mod mdparser;
pub mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::task::Task as TaskTrait;
use async_trait::async_trait;

const PROVIDER_NAME: &str = "Obsidian";

pub struct Provider {
    c: client::Client,
}

impl Provider {
    pub fn new(path: &str) -> Self {
        Self {
            c: client::Client::new(path),
        }
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.c.path()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &mut self,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn std::error::Error>> {
        let tasks = self.c.tasks(f).await?;
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for t in tasks {
            result.push(Box::new(t));
        }
        Ok(result)
    }
    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}
