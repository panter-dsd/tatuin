use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition, Value};
use std::{error::Error, path::Path, sync::Arc};
use tatuin_core::project::Project as ProjectTrait;

use super::project::{Project, inbox_project};

const DB_FILE_NAME: &str = "tatuin.db";
const PROJECTS_TABLE: TableDefinition<&str, Project> = TableDefinition::new("projects");
const TASKS_TABLE: TableDefinition<&str, u64> = TableDefinition::new("tasks");

impl Value for Project {
    type SelfType<'a>
        = Project
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap_or_default()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Vec<u8>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap_or_default()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Project")
    }
}

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
