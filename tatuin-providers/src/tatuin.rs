// SPDX-License-Identifier: MIT

mod client;
mod project;

use std::error::Error;

use async_trait::async_trait;
use client::Client;
use tatuin_core::{
    StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProviderTrait},
    task::Task as TaskTrait,
    task_patch::{PatchError, TaskPatch},
};

use crate::config::Config;

pub const PROVIDER_NAME: &str = "Tatuin";

pub struct Provider {
    cfg: Config,
    c: Client,
}

impl Provider {
    pub fn new(cfg: Config) -> Result<Self, Box<dyn Error>> {
        let c = Client::new(&cfg.cache_path(PROVIDER_NAME)?)?;
        Ok(Self { cfg, c })
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
        self.cfg.name()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        todo!("Implement me")
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        self.c
            .projects()
            .await
            .map(|v| v.iter().map(|p| p.clone_boxed()).collect())
            .map_err(|e| {
                tracing::error!(error=?e, "Get projects from database");
                e.into()
            })
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        todo!("Implement me")
    }

    async fn reload(&mut self) {
        // do nothing for now
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: true }
    }

    async fn create_task(&mut self, _project_id: &str, tp: &TaskPatch) -> Result<(), StringError> {
        todo!("Implement me")
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use tatuin_core::{project::Project, provider::ProviderTrait};

    use crate::{config::Config, tatuin::project::inbox_project};

    use super::Provider;

    fn config(cache_path: PathBuf) -> Config {
        let mut cfg = Config::new("test_app", "test_name");
        cfg.cache_path = cache_path;
        cfg
    }

    #[tokio::test]
    async fn get_projects_on_empty_provider() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let p = Provider::new(config(temp_dir.path().to_path_buf()));
        assert!(p.is_ok());

        let p: &mut dyn ProviderTrait = &mut p.unwrap();

        let projects = p.projects().await;
        assert!(projects.is_ok());

        let projects = projects.unwrap();
        assert_eq!(projects.len(), 1);

        let project = &projects[0];
        let inbox = inbox_project();
        assert_eq!(project.name(), inbox.name());
        assert_eq!(project.description(), inbox.description());
        assert_eq!(project.provider(), inbox.provider());
        assert_eq!(project.parent_id(), inbox.parent_id());
        assert_eq!(project.is_inbox(), inbox.is_inbox());
        assert_eq!(project.is_favorite(), inbox.is_favorite());
    }
}
