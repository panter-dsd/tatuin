use crate::task;
use reqwest::header::HeaderMap;
use serde::Deserialize;

const BASE_URL: &str = "https://todoist.com/api/v1";

pub struct Task {
    pub id: String,
    pub text: String,
    pub checked: bool,
}

impl task::Task for Task {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn text(&self) -> String {
        self.text.to_string()
    }

    fn state(&self) -> task::State {
        if self.checked {
            task::State::Completed
        } else {
            task::State::Uncompleted
        }
    }
}

pub struct Project {
    pub id: String,
    pub name: String,
}

#[derive(Debug)]
pub struct TaskFilter {
    pub project: Option<String>,
}

pub struct Todoist {
    default_header: HeaderMap,
    client: reqwest::Client,
}

impl Todoist {
    pub fn new(api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", api_key).parse().unwrap(),
        );
        Self {
            default_header: headers,
            client: reqwest::Client::new(),
        }
    }

    pub async fn tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut result: Vec<Task> = Vec::new();

        let mut cursor = None;

        loop {
            let resp = self
                .client
                .get(format!("{BASE_URL}/tasks{}", task_query(&filter, &cursor)))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<get_tasks::Response>()
                .await?;

            for t in resp.results {
                result.push(Task {
                    id: t.id,
                    text: t.content,
                    checked: t.checked,
                })
            }

            if resp.next_cursor.is_none() {
                break;
            }

            cursor = resp.next_cursor;
        }

        Ok(result)
    }

    pub async fn projects(&self) -> Result<Vec<Project>, Box<dyn std::error::Error>> {
        let mut result: Vec<Project> = Vec::new();

        let mut cursor = None;

        loop {
            let mut query: String = String::from("?limit=200");
            if let Some(c) = cursor {
                query.push_str(format!("&cursor={c}").as_str());
            }

            let resp = self
                .client
                .get(format!("{BASE_URL}/projects{query}"))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<get_projects::Response>()
                .await?;

            for p in resp.results {
                result.push(Project {
                    id: p.id,
                    name: p.name,
                })
            }

            if resp.next_cursor.is_none() {
                break;
            }

            cursor = resp.next_cursor;
        }

        Ok(result)
    }
}

fn task_query(filter: &TaskFilter, cursor: &Option<String>) -> String {
    let mut query: Vec<String> = Vec::new();
    query.push(String::from("limit=200"));

    if let Some(p) = &filter.project {
        query.push(format!("project_id={p}"));
    }
    if let Some(c) = cursor {
        query.push(format!("cursor={c}"));
    }

    format!("?{}", query.join("&"))
}

mod get_tasks {
    use super::*;

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Duration {
        property1: Option<String>,
        property2: Option<String>,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Task {
        pub id: String,
        pub user_id: String,
        pub project_id: String,
        pub section_id: Option<String>,
        pub parent_id: Option<String>,
        pub added_by_uid: Option<String>,
        pub assigned_by_uid: Option<String>,
        pub responsible_uid: Option<String>,
        pub labels: Vec<String>,
        pub deadline: Option<Duration>,
        pub duration: Option<Duration>,
        pub checked: bool,
        pub is_deleted: bool,
        pub added_at: Option<String>,
        pub completed_at: Option<String>,
        pub updated_at: Option<String>,
        // due: ???,
        pub priority: i32,
        pub child_order: i32,
        pub content: String,
        pub description: String,
        pub note_count: i32,
        pub day_order: i32,
        pub is_collapsed: bool,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Response {
        pub results: Vec<Task>,
        pub next_cursor: Option<String>,
    }
}

mod get_projects {
    use super::*;

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Project {
        pub id: String,
        pub can_assign_tasks: bool,
        pub child_order: i32,
        pub color: String,
        pub created_at: Option<String>,
        pub is_archived: bool,
        pub is_deleted: bool,
        pub is_favorite: bool,
        pub is_frozen: bool,
        pub name: String,
        pub updated_at: Option<String>,
        pub view_style: String,
        pub default_order: i32,
        pub description: String,
        pub parent_id: Option<String>,
        pub inbox_project: bool,
        pub is_collapsed: bool,
        pub is_shared: bool,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Response {
        pub results: Vec<Project>,
        pub next_cursor: Option<String>,
    }
}
