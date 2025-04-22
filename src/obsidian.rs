use std::fs;
use std::path::Path;
use std::sync::Arc;
mod mdparser;
pub mod task;
use crate::filter::{Filter, FilterState};
use task::Task;
use tokio::sync::Semaphore;

use crate::project::Project as ProjectTrait;
use crate::task::Provider as ProviderTrait;
use crate::task::Task as TaskTrait;
use async_trait::async_trait;

const SIMULTANEOUS_JOB_COUNT: usize = 10;
const PROVIDER_NAME: &str = "Obsidian";

pub struct Client {
    path: String,
}

impl Client {
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
        }
    }

    pub fn all_supported_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        supported_files(Path::new(self.path.as_str()))
    }

    pub async fn tasks(&self, f: &Filter) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let files = self.all_supported_files()?;

        let mut tasks: Vec<Task> = Vec::new();

        let semaphore = Arc::new(Semaphore::new(SIMULTANEOUS_JOB_COUNT));

        let mut jobs = Vec::new();

        for f in files {
            let semaphore = semaphore.clone();
            let p = self.path.clone();

            let job = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                let parser = mdparser::Parser::new(f.as_str());
                let mut tasks = parser.tasks().await.unwrap();
                for t in &mut tasks {
                    t.set_root_path(p.to_string());
                }
                drop(_permit);
                tasks
            });

            jobs.push(job);
        }

        for job in jobs {
            let mut response = job
                .await
                .unwrap()
                .iter()
                .filter(|t| accept_filter(t, f))
                .cloned()
                .collect::<Vec<Task>>();

            tasks.append(&mut response);
        }

        Ok(tasks)
    }
}

const fn state_to_list_state(s: &task::State) -> FilterState {
    match s {
        task::State::Completed => FilterState::Completed,
        task::State::Uncompleted => FilterState::Uncompleted,
        task::State::InProgress => FilterState::InProgress,
        task::State::Unknown(_) => FilterState::Unknown,
    }
}

fn accept_filter(t: &Task, f: &Filter) -> bool {
    if !f.states.contains(&state_to_list_state(&t.state)) {
        return false;
    }

    if f.today {
        if let Some(d) = t.due {
            let now = chrono::Utc::now().date_naive();
            if d.date_naive() != now {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn supported_files(p: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut result: Vec<String> = Vec::new();

    for e in fs::read_dir(p)? {
        let entry = e?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        if path.is_file() && name.ends_with(".md") {
            if let Some(p) = path.to_str() {
                result.push(String::from(p));
            }
        } else if path.is_dir() {
            let mut files = supported_files(path.as_path())?;
            result.append(&mut files);
        }
    }

    Ok(result)
}

pub struct Provider {
    c: Client,
}

impl Provider {
    pub fn new(c: Client) -> Self {
        Self { c }
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.c.path.to_string()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &self,
        f: &Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn std::error::Error>> {
        let tasks = self.c.tasks(f).await?;
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for t in tasks {
            result.push(Box::new(t));
        }
        Ok(result)
    }
    async fn projects(&self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}
