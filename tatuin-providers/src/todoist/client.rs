// SPDX-License-Identifier: MIT

use super::{project::Project, task::Task};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tatuin_core::{filter, project::Project as ProjectTrait};
use url::Url;
use url_builder::URLBuilder;
use urlencoding::encode;

const BASE_URL: &str = "https://todoist.com/api/v1";

#[derive(Debug, Serialize)]
pub struct CreateTaskRequest<'a> {
    pub content: &'a str,
    pub description: Option<&'a str>,
    pub project_id: Option<&'a str>,
    pub due_string: Option<&'a str>,
    pub priority: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UpdateTaskRequest<'a> {
    pub content: Option<&'a str>,
    pub description: Option<&'a str>,
    pub due_string: Option<&'a str>,
    pub priority: Option<i32>,
}

pub struct Client {
    default_header: HeaderMap,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl Client {
    pub fn new(api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {api_key}").parse().unwrap());
        Self {
            default_header: headers,
            client: reqwest::Client::new(),
        }
    }

    pub async fn completed_tasks(
        &self,
        project_id: &Option<String>,
        f: &filter::Filter,
    ) -> Result<Vec<Task>, Box<dyn Error>> {
        let mut result: Vec<Task> = Vec::new();

        let mut cursor = None;

        let query = {
            let mut v = vec![
                String::from("limit=200"),
                format!(
                    "since={}",
                    chrono::Utc::now()
                        .checked_sub_days(chrono::Days::new(7))
                        .unwrap()
                        .format("%Y-%m-%dT%H:%M:%SZ")
                ),
                format!("until={}", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")),
                format!("filter_query={}", filter_to_query(&None, f)),
            ];

            if let Some(p) = project_id {
                v.push(format!("project_id={p}"));
            }

            v
        };

        #[derive(Deserialize)]
        struct Response {
            items: Vec<Task>,
            next_cursor: Option<String>,
        }

        loop {
            let mut q = query.clone();
            if let Some(c) = cursor {
                q.push(format!("cursor={c}"));
            }
            let mut resp = self
                .client
                .get(format!("{BASE_URL}/tasks/completed?{}", &q.join("&")))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<Response>()
                .await?;

            result.append(&mut resp.items);

            if resp.next_cursor.is_none() {
                break;
            }

            cursor = resp.next_cursor;
        }

        Ok(result)
    }

    pub async fn tasks_by_filter(
        &self,
        project: &Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Task>, Box<dyn Error>> {
        let mut result: Vec<Task> = Vec::new();

        let u = Url::parse(BASE_URL).unwrap();
        let mut cursor: Option<String> = None;

        let mut project_name = None;
        if let Some(p) = project {
            project_name = Some(p.name())
        }

        #[derive(Deserialize, Debug)]
        struct Response {
            pub results: Vec<Task>,
            pub next_cursor: Option<String>,
        }

        loop {
            let mut url = URLBuilder::new();
            url.set_protocol(u.scheme())
                .set_host(u.host_str().unwrap_or_default())
                .set_port(u.port().unwrap_or_default())
                .add_route("api/v1/tasks/filter")
                .add_param("limit", "200")
                .add_param("query", filter_to_query(&project_name, f).as_str());

            if let Some(c) = cursor {
                url.add_param("cursor", c.as_str());
            }
            let built_url = url.build();

            let mut resp = self
                .client
                .get(built_url)
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<Response>()
                .await?;

            result.append(&mut resp.results);

            if resp.next_cursor.is_none() {
                break;
            }

            cursor = resp.next_cursor;
        }

        Ok(result)
    }

    pub async fn projects(&self) -> Result<Vec<Project>, Box<dyn Error>> {
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

    pub async fn project(&self, id: &str) -> Result<Project, Box<dyn Error>> {
        let resp = self
            .client
            .get(format!("{BASE_URL}/projects/{id}"))
            .headers(self.default_header.clone())
            .send()
            .await?
            .json::<Project>()
            .await?;

        Ok(resp)
    }

    pub async fn close_task(&self, task_id: &str) -> Result<(), Box<dyn Error>> {
        let resp = self
            .client
            .post(format!("{BASE_URL}/tasks/{task_id}/close"))
            .headers(self.default_header.clone())
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(());
        }
        Err(Box::<dyn Error>::from(format!(
            "wrong status: {}",
            resp.status().as_str()
        )))
    }

    pub async fn reopen_task(&self, task_id: &str) -> Result<(), Box<dyn Error>> {
        self.client
            .post(format!("{BASE_URL}/tasks/{task_id}/reopen"))
            .headers(self.default_header.clone())
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"todoist_client", task_id=?task_id, error=?e, "Reopen the task");
                Box::<dyn Error>::from(e.to_string())
            })
    }

    pub async fn update_task(&self, task_id: &str, r: &UpdateTaskRequest<'_>) -> Result<(), Box<dyn Error>> {
        self.client
            .post(format!("{BASE_URL}/tasks/{task_id}"))
            .json(r)
            .headers(self.default_header.clone())
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"todoist_client", request=?r, error=?e, "Update the task");
                Box::<dyn Error>::from(e.to_string())
            })
    }

    pub async fn create_task(&self, r: &CreateTaskRequest<'_>) -> Result<(), Box<dyn Error>> {
        self.client
            .post(format!("{BASE_URL}/tasks"))
            .json(r)
            .headers(self.default_header.clone())
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"todoist_client", request=?r, error=?e, "Create the task");
                Box::<dyn Error>::from(e.to_string())
            })
    }

    pub async fn delete_task(&self, task_id: &str) -> Result<(), Box<dyn Error>> {
        if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Invalid task ID format".into());
        }

        let url = format!("{BASE_URL}/tasks/{task_id}");
        self.client
            .delete(url)
            .headers(self.default_header.clone())
            .send()
            .await?
            .error_for_status()
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"todoist_client", task_id=task_id, error=?e, "Delete the task");
                Box::<dyn Error>::from(e.to_string())
            })
    }
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

fn filter_to_query(project_name: &Option<String>, f: &filter::Filter) -> String {
    let mut and_filter = Vec::new();
    let mut todoist_query: Vec<&str> = Vec::new();

    if f.due.contains(&filter::Due::Today) {
        todoist_query.push("today");
    }

    if f.due.contains(&filter::Due::Overdue) {
        todoist_query.push("overdue");
    }

    if f.due.contains(&filter::Due::NoDate) {
        todoist_query.push("no date");
    }

    if !todoist_query.is_empty() {
        and_filter.push(format!("({})", todoist_query.join("|")));
    }

    if let Some(p) = project_name {
        and_filter.push(format!("#{p}"));
    }

    encode(and_filter.join("&").as_str()).into_owned()
}
