use reqwest::header::HeaderMap;
use serde::Deserialize;
mod task;
use task::Task;
mod project;
use project::Project;

use crate::filter;

const BASE_URL: &str = "https://todoist.com/api/v1";

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

    pub async fn tasks(
        &self,
        project: &Option<String>,
        _f: &filter::Filter,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut result: Vec<Task> = Vec::new();

        let mut cursor = None;

        loop {
            let mut resp = self
                .client
                .get(format!("{BASE_URL}/tasks{}", task_query(project, &cursor)))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<TaskResponse>()
                .await?;

            result.append(&mut resp.results);

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

            let mut resp = self
                .client
                .get(format!("{BASE_URL}/projects{query}"))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<ProjectResponse>()
                .await?;

            result.append(&mut resp.results);

            if resp.next_cursor.is_none() {
                break;
            }

            cursor = resp.next_cursor;
        }

        Ok(result)
    }
}

fn task_query(project: &Option<String>, cursor: &Option<String>) -> String {
    let mut query: Vec<String> = Vec::new();
    query.push(String::from("limit=200"));

    if let Some(p) = project {
        query.push(format!("project_id={p}"));
    }
    if let Some(c) = cursor {
        query.push(format!("cursor={c}"));
    }

    format!("?{}", query.join("&"))
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct TaskResponse {
    pub results: Vec<Task>,
    pub next_cursor: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ProjectResponse {
    pub results: Vec<Project>,
    pub next_cursor: Option<String>,
}
