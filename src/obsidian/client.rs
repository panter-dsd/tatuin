use crate::filter;
use crate::obsidian::md_file;
use crate::obsidian::task::{State, Task};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

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

    pub fn all_supported_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        supported_files(Path::new(self.path.as_str()))
    }

    pub async fn tasks(&self, f: &filter::Filter) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let files = self.all_supported_files()?;

        let mut tasks: Vec<Task> = Vec::new();

        let semaphore = Arc::new(Semaphore::new(SIMULTANEOUS_JOB_COUNT));

        let mut jobs = Vec::new();

        for f in files {
            let semaphore = semaphore.clone();
            let p = self.path.clone();

            let job = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                let parser = md_file::File::new(f.as_str());
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

const fn state_to_list_state(s: &State) -> filter::FilterState {
    match s {
        State::Completed => filter::FilterState::Completed,
        State::Uncompleted => filter::FilterState::Uncompleted,
        State::InProgress => filter::FilterState::InProgress,
        State::Unknown(_) => filter::FilterState::Unknown,
    }
}

fn accept_filter(t: &Task, f: &filter::Filter) -> bool {
    if !f.states.contains(&state_to_list_state(&t.state)) {
        return false;
    }

    let due = match t.due {
        Some(d) => {
            let now = chrono::Utc::now().date_naive();
            match d.date_naive().cmp(&now) {
                Ordering::Less => filter::Due::Overdue,
                Ordering::Equal => filter::Due::Today,
                Ordering::Greater => filter::Due::Future,
            }
        }
        None => filter::Due::NoDate,
    };

    if !f.due.contains(&due) {
        return false;
    }

    true
}
