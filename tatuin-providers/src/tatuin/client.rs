use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::{error::Error, path::Path, sync::Arc};
use tatuin_core::{
    filter::{Filter, FilterState},
    project::Project as ProjectTrait,
    task::due_group,
};

use super::{
    project::{Project, inbox_project},
    task::Task,
};

const DB_FILE_NAME: &str = "tatuin.db";
const PROJECTS_TABLE: TableDefinition<&str, Project> = TableDefinition::new("projects");
const TASKS_TABLE: TableDefinition<&str, Task> = TableDefinition::new("tasks");
const COMPLETED_TASKS_TABLE: TableDefinition<&str, Task> = TableDefinition::new("completed_tasks");

pub struct Client {
    db: Arc<Database>,
}

impl Client {
    pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            db: Arc::new(Database::create(path.join(DB_FILE_NAME))?),
        })
    }

    pub async fn projects(&self) -> Result<Vec<Project>, Box<dyn Error>> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || projects(&db))
            .await?
            .map_err(|e| e as Box<dyn Error>)
    }

    pub async fn tasks(&self, project_id: Option<uuid::Uuid>, f: &Filter) -> Result<Vec<Task>, Box<dyn Error>> {
        let db = Arc::clone(&self.db);
        let f = f.clone();
        tokio::task::spawn_blocking(move || tasks(&db, project_id, f))
            .await?
            .map_err(|e| e as Box<dyn Error>)
    }
}

fn projects(db: &Database) -> Result<Vec<Project>, Box<dyn Error + Send + Sync>> {
    {
        let tx = db.begin_read()?;
        let table = tx.open_table(PROJECTS_TABLE);
        if let Ok(table) = table
            && !table.is_empty()?
        {
            let mut result = Vec::new();
            for v in table.iter()? {
                result.push(v?.1.value());
            }
            return Ok(result);
        }
    }
    init_projects_table(db)?;
    projects(db)
}

fn init_projects_table(db: &Database) -> Result<(), Box<dyn Error + Send + Sync>> {
    let tx = db.begin_write()?;
    {
        let mut table = tx.open_table(PROJECTS_TABLE)?;

        let p = inbox_project();
        table.insert(p.id().as_str(), p)?;
    }
    tx.commit()?;
    Ok(())
}

fn tasks(db: &Database, project_id: Option<uuid::Uuid>, f: Filter) -> Result<Vec<Task>, Box<dyn Error + Send + Sync>> {
    let tx = db.begin_read()?;
    let mut result = Vec::new();
    let accept_filter = |t: &Task| -> bool {
        project_id.is_none_or(|id| t.project_id == id)
            && f.states.contains(&t.state.into())
            && f.due.contains(&due_group(&t.due))
    };

    if f.states.contains(&FilterState::Completed) {
        let table = tx.open_table(COMPLETED_TASKS_TABLE);
        if let Ok(table) = table {
            for v in table.iter()? {
                let t = v?.1.value();
                if accept_filter(&t) {
                    result.push(t);
                }
            }
        }
    }

    if !f.states.iter().any(|s| s != &FilterState::Completed) {
        let table = tx.open_table(TASKS_TABLE);
        if let Ok(table) = table {
            for v in table.iter()? {
                let t = v?.1.value();
                if accept_filter(&t) {
                    result.push(t);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use tatuin_core::project::Project;

    use super::Client;

    #[tokio::test]
    async fn database_file_exists() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");
        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 0);

        let _ = Client::new(temp_dir.path()).unwrap();

        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn check_inbox_creates_once() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let c = Client::new(temp_dir.path()).unwrap();

        let projects = c.projects().await.unwrap();
        let project1 = projects[0].clone();

        let projects = c.projects().await.unwrap();
        let project2 = projects[0].clone();

        assert_eq!(project2, project1);
    }

    #[tokio::test]
    async fn db_stores_to_disk_and_doesnt_recreate() {
        let temp_dir = tempfile::tempdir().expect("Can't create a temp dir");

        let project1 = {
            let c = Client::new(temp_dir.path()).unwrap();
            let projects = c.projects().await.unwrap();
            projects[0].clone()
        };

        let project2 = {
            let c = Client::new(temp_dir.path()).unwrap();
            let projects = c.projects().await.unwrap();
            projects[0].clone()
        };

        assert_eq!(project2.id(), project1.id());
    }
}
