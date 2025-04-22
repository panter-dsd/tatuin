use crate::filter;
use crate::project;
use crate::task;
use async_trait::async_trait;

#[async_trait]
pub trait Provider {
    fn name(&self) -> String;
    fn type_name(&self) -> String;
    async fn tasks(
        &self,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn task::Task>>, Box<dyn std::error::Error>>;
    async fn projects(&self) -> Result<Vec<Box<dyn project::Project>>, Box<dyn std::error::Error>>;
}
