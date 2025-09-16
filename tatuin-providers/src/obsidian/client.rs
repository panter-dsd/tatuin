// SPDX-License-Identifier: MIT

use super::patch::{PatchError, TaskPatch};
use crate::obsidian::md_file;
use crate::obsidian::task::Task;
use itertools::Itertools;
use std::cmp::Ordering;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tatuin_core::filter;
use tokio::sync::Semaphore;
use tracing::Level;

const SIMULTANEOUS_JOB_COUNT: usize = 10;

pub struct Client {
    path: String,
}

impl Client {
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
        }
    }

    pub fn root_path(&self) -> String {
        self.path.clone()
    }

    pub fn all_supported_files(&self) -> Result<Vec<String>, Box<dyn Error>> {
        supported_files(Path::new(self.path.as_str()))
    }

    pub async fn tasks(&self, f: &filter::Filter) -> Result<Vec<Task>, Box<dyn Error>> {
        let span = tracing::span!(Level::TRACE, "tasks", path=self.path,  filter = ?&f, "Load tasks");
        let _enter = span.enter();

        let files = self.all_supported_files()?;

        let mut tasks: Vec<Task> = Vec::new();

        let semaphore = Arc::new(Semaphore::new(SIMULTANEOUS_JOB_COUNT));

        let mut jobs = Vec::new();

        for f in files {
            let semaphore = semaphore.clone();
            let p = self.path.clone();

            let job = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                let mut tasks = Vec::new();

                let mut parser = md_file::File::new(f.as_str());
                if parser.open().is_ok() {
                    tasks = parser.tasks().await.unwrap();
                    for t in &mut tasks {
                        t.set_root_path(p.to_string());
                    }
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
                .filter(|t| f.accept(*t))
                .cloned()
                .collect::<Vec<Task>>();

            tasks.append(&mut response);
        }

        drop(_enter);
        Ok(tasks)
    }

    pub async fn patch_tasks<'a>(&mut self, patches: &'a [TaskPatch<'a>]) -> Vec<PatchError> {
        let mut errors = Vec::new();

        let mut files: Vec<&'a str> = Vec::new();
        for p in patches {
            files.push(&p.task.file_path);
        }

        for file in files.iter().unique() {
            let mut f = md_file::File::new(file);
            if let Err(e) = f.open() {
                errors.extend(
                    patches
                        .iter()
                        .filter(|p| p.task.file_path.as_str().cmp(file) == Ordering::Equal)
                        .map(|p| PatchError {
                            task: p.task.clone(),
                            error: e.to_string(),
                        }),
                );
                continue;
            }
            let mut file_patches = patches
                .iter()
                .filter(|p| p.task.file_path.as_str().cmp(file) == Ordering::Equal)
                .collect::<Vec<&'a TaskPatch>>();
            file_patches.sort_by_key(|p| std::cmp::Reverse(p.task.start_pos));
            for p in file_patches {
                if let Err(e) = f.patch_task(p).await {
                    errors.push(PatchError {
                        task: p.task.clone(),
                        error: e.to_string(),
                    });
                }
            }
            if let Err(e) = f.flush() {
                errors.extend(
                    patches
                        .iter()
                        .filter(|p| p.task.file_path.as_str().cmp(file) == Ordering::Equal)
                        .map(|p| PatchError {
                            task: p.task.clone(),
                            error: e.to_string(),
                        }),
                );
            }
        }

        errors
    }
}

fn supported_files(p: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut result: Vec<String> = Vec::new();

    for e in fs::read_dir(p)? {
        let entry = e?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_str().unwrap_or_default();
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
