pub mod client;
mod mdparser;
pub mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::task::{State, Task as TaskTrait};
use async_trait::async_trait;
use std::error::Error;

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
    fn id(&self) -> String {
        todo!("implement me")
    }

    fn name(&self) -> String {
        self.c.path()
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
        for t in tasks {
            result.push(Box::new(t));
        }
        Ok(result)
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>> {
        Ok(Vec::new())
    }

    async fn change_task_state(
        &mut self,
        _task: Box<dyn TaskTrait>,
        _state: State,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
