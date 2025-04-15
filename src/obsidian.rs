use std::fs;
use std::path::Path;
use std::sync::Arc;
mod mdparser;
pub mod task;
use task::Task;
use tokio::sync::Semaphore;

const SIMULTANEOUS_JOB_COUNT: usize = 10;

pub struct Obsidian {
    path: String,
}

impl Obsidian {
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
        }
    }

    pub fn count(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let files = self.all_supported_files()?;
        Ok(files.len() as u64)
    }

    pub fn all_supported_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        supported_files(Path::new(self.path.as_str()))
    }

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let files = self.all_supported_files()?;

        let mut tasks: Vec<Task> = Vec::new();

        let semaphore = Arc::new(Semaphore::new(SIMULTANEOUS_JOB_COUNT));

        let mut jobs = Vec::new();
        for f in files {
            let semaphore = semaphore.clone();
            let file_name = String::from(f.as_str());
            let job = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let parser = mdparser::Parser::new(file_name.as_str());
                let tasks = parser.tasks().await.unwrap();
                drop(_permit);
                tasks
            });
            jobs.push(job);
        }

        for job in jobs {
            let mut response = job.await.unwrap();
            tasks.append(&mut response);
        }
        Ok(tasks)
    }
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
