use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use tatuin_core::{
    filter::{Filter, FilterState},
    project::Project as ProjectTrait,
    task::{State, Task as TaskTrait, due_group},
    task_patch::PatchError,
};

use super::{
    project::{Project, inbox_project},
    task::Task,
};

type SyncedError = Box<dyn Error + Send + Sync>;

const DB_FILE_NAME: &str = "tatuin.db";
const PROJECTS_TABLE: TableDefinition<&str, Project> = TableDefinition::new("projects");
const TASKS_TABLE: TableDefinition<&str, Task> = TableDefinition::new("tasks");
const COMPLETED_TASKS_TABLE: TableDefinition<&str, Task> = TableDefinition::new("completed_tasks");

pub struct Client {
    path: PathBuf,
}

impl Client {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    pub async fn projects(&self, provider_name: &str) -> Result<Vec<Project>, Box<dyn Error>> {
        let db = Database::create(self.path.join(DB_FILE_NAME))?;
        let provider_name = provider_name.to_string();
        tokio::task::spawn_blocking(move || projects(&db, &provider_name))
            .await?
            .map_err(|e| e as Box<dyn Error>)
    }

    pub async fn tasks(&self, project_id: Option<uuid::Uuid>, f: &Filter) -> Result<Vec<Task>, Box<dyn Error>> {
        let db = Database::create(self.path.join(DB_FILE_NAME))?;
        let f = f.clone();
        tokio::task::spawn_blocking(move || tasks(&db, project_id, f))
            .await?
            .map_err(|e| e as Box<dyn Error>)
    }

    pub async fn create_task(&self, t: Task) -> Result<(), Box<dyn Error>> {
        let db = Database::create(self.path.join(DB_FILE_NAME))?;
        tokio::task::spawn_blocking(move || create_task(&db, t))
            .await?
            .map_err(|e| e as Box<dyn Error>)
    }

    pub async fn patch_tasks(&self, tasks: &[Task]) -> Vec<PatchError> {
        let db = Database::create(self.path.join(DB_FILE_NAME));
        if let Err(e) = db {
            return fill_global_error(Vec::new(), tasks, e.to_string().as_str());
        }

        let db = db.unwrap();
        let tasks_copy = tasks.to_vec();
        match tokio::task::spawn_blocking(move || patch_tasks(&db, &tasks_copy)).await {
            Ok(v) => v,
            Err(e) => fill_global_error(Vec::new(), tasks, e.to_string().as_str()),
        }
    }
}

fn projects(db: &Database, provider_name: &str) -> Result<Vec<Project>, SyncedError> {
    {
        let tx = db.begin_read()?;
        let table = tx.open_table(PROJECTS_TABLE);
        if let Ok(table) = table
            && !table.is_empty()?
        {
            let mut result = Vec::new();
            for v in table.iter()? {
                let mut p = v?.1.value();
                p.set_provider_name(provider_name);
                result.push(p);
            }
            return Ok(result);
        }
    }
    init_projects_table(db, provider_name)?;
    projects(db, provider_name)
}

fn init_projects_table(db: &Database, provider_name: &str) -> Result<(), SyncedError> {
    let tx = db.begin_write()?;
    {
        let mut table = tx.open_table(PROJECTS_TABLE)?;

        let p = inbox_project(provider_name);
        table.insert(p.id().as_str(), p)?;
    }
    tx.commit()?;
    Ok(())
}

fn tasks(db: &Database, project_id: Option<uuid::Uuid>, f: Filter) -> Result<Vec<Task>, SyncedError> {
    let tx = db.begin_read()?;
    let mut result = Vec::new();

    let accept_filter = |t: &Task| -> bool {
        project_id.is_none_or(|id| t.project_id == id)
            && f.states.contains(&t.state.into())
            && f.due.contains(&due_group(&t.due))
    };

    let mut load_tasks = |td: TableDefinition<&str, Task>| -> Result<(), SyncedError> {
        let table = tx.open_table(td);
        if let Ok(table) = table {
            for v in table.iter()? {
                let t = v?.1.value();
                if accept_filter(&t) {
                    result.push(t);
                }
            }
        }

        Ok(())
    };

    if f.states.contains(&FilterState::Completed) {
        load_tasks(COMPLETED_TASKS_TABLE)?;
    }

    if f.states.iter().any(|s| s != &FilterState::Completed) {
        load_tasks(TASKS_TABLE)?;
    }

    Ok(result)
}

fn create_task(db: &Database, t: Task) -> Result<(), SyncedError> {
    let tx = db.begin_write()?;
    {
        let mut table = tx.open_table(TASKS_TABLE)?;
        table.insert(t.id.to_string().as_str(), t)?;
    }
    tx.commit()?;
    Ok(())
}

fn fill_global_error(errors: Vec<PatchError>, tasks: &[Task], error: &str) -> Vec<PatchError> {
    [
        errors,
        tasks
            .iter()
            .map(|t| PatchError {
                task: t.clone_boxed(),
                error: error.to_string(),
            })
            .collect::<Vec<PatchError>>(),
    ]
    .concat()
}

fn patch_tasks(db: &Database, tasks: &[Task]) -> Vec<PatchError> {
    let mut errors = Vec::new();

    let tx = db.begin_write();
    if let Err(e) = &tx {
        return fill_global_error(errors, tasks, e.to_string().as_str());
    }

    let tx = tx.unwrap();

    {
        let tasks_table = tx.open_table(TASKS_TABLE);
        if let Err(e) = &tasks_table {
            return fill_global_error(errors, tasks, e.to_string().as_str());
        }
        let mut tasks_table = tasks_table.unwrap();

        let completed_tasks_table = tx.open_table(COMPLETED_TASKS_TABLE);
        if let Err(e) = &completed_tasks_table {
            return fill_global_error(errors, tasks, e.to_string().as_str());
        }
        let mut completed_tasks_table = completed_tasks_table.unwrap();

        for t in tasks {
            let id = t.id().to_string();
            if let Err(e) = tasks_table.remove(id.as_str()) {
                tracing::error!(error=?e, id=id, "Remove task from tasks table");
            }
            if let Err(e) = completed_tasks_table.remove(id.as_str()) {
                tracing::error!(error=?e, id=id, "Remove task from completed tasks table");
            }
            let result = if t.state == State::Completed {
                completed_tasks_table.insert(id.as_str(), t)
            } else {
                tasks_table.insert(id.as_str(), t)
            };
            if let Err(e) = result {
                tracing::error!(error=?e, id=id, "Add the task to database");
                errors.push(PatchError {
                    task: t.clone_boxed(),
                    error: e.to_string(),
                });
            }
        }
    }
    if let Err(e) = tx.commit() {
        return fill_global_error(errors, tasks, e.to_string().as_str());
    }

    errors
}

#[cfg(test)]
mod test {
    use tatuin_core::project::Project;

    use super::Client;

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn database_file_exists() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");
        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 0);

        let c = Client::new(temp_dir.path());
        let _ = c.projects("test_name").await;

        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn check_inbox_creates_once() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let c = Client::new(temp_dir.path());

        let projects = c.projects("test_name").await.unwrap();
        let project1 = projects[0].clone();

        let projects = c.projects("test_name").await.unwrap();
        let project2 = projects[0].clone();

        assert_eq!(project2, project1);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn db_stores_to_disk_and_doesnt_recreate() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let project1 = {
            let c = Client::new(temp_dir.path());
            let projects = c.projects("test_name").await.unwrap();
            projects[0].clone()
        };

        let project2 = {
            let c = Client::new(temp_dir.path());
            let projects = c.projects("test_name").await.unwrap();
            projects[0].clone()
        };

        assert_eq!(project2.id(), project1.id());
    }
}
