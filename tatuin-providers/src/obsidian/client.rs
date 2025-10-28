// SPDX-License-Identifier: MIT

use crate::obsidian::{
    fs, md_file,
    patch::{PatchError, TaskPatch},
    task::Task,
};
use itertools::Itertools;
use std::cmp::Ordering;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tatuin_core::filter;
use tokio::sync::Semaphore;
use tracing::Level;

const SIMULTANEOUS_JOB_COUNT: usize = 10;

pub struct Client {
    path: PathBuf,
}

impl Client {
    pub fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    pub fn root_path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn all_supported_files(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        fs::supported_files(&self.path)
    }

    pub async fn tasks(&self, f: &filter::Filter) -> Result<Vec<Task>, Box<dyn Error>> {
        let span = tracing::span!(Level::TRACE, "tasks", path=?self.path,  filter = ?&f, "Load tasks");
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

                let mut parser = md_file::File::new(&f);
                if parser.open().is_ok() {
                    tasks = parser.tasks().await.unwrap();
                    for t in &mut tasks {
                        t.set_vault_path(&p);
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

        let mut files: Vec<&'a Path> = Vec::new();
        for p in patches {
            files.push(&p.task.file_path);
        }

        for file in files.iter().unique() {
            let mut f = md_file::File::new(file);
            if let Err(e) = f.open() {
                errors.extend(
                    patches
                        .iter()
                        .filter(|p| p.task.file_path.cmp(&file.to_path_buf()) == Ordering::Equal)
                        .map(|p| PatchError {
                            task: p.task.clone(),
                            error: e.to_string(),
                        }),
                );
                continue;
            }
            let mut file_patches = patches
                .iter()
                .filter(|p| p.task.file_path.cmp(&file.to_path_buf()) == Ordering::Equal)
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
                        .filter(|p| p.task.file_path.cmp(&file.to_path_buf()) == Ordering::Equal)
                        .map(|p| PatchError {
                            task: p.task.clone(),
                            error: e.to_string(),
                        }),
                );
            }
        }

        errors
    }

    pub async fn delete_task(&mut self, t: &Task) -> Result<(), Box<dyn Error>> {
        let mut f = md_file::File::new(&t.file_path);
        f.open()?;
        f.delete_task(t).await?;
        f.flush()
    }
}
