use crate::filter;
use crate::todoist::project::Project;
use crate::todoist::task::Task;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use url::Url;
use url_builder::URLBuilder;
use urlencoding::encode;

const BASE_URL: &str = "https://todoist.com/api/v1";

pub struct Client {
    default_header: HeaderMap,
    client: reqwest::Client,
}

impl Client {
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

    pub async fn completed_tasks(
        &self,
        project_id: &Option<String>,
        f: &filter::Filter,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
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
                .get(format!(
                    "{BASE_URL}/tasks/completed/by_completion_date?{}",
                    &q.join("&")
                ))
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
        project_name: &Option<String>,
        f: &filter::Filter,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut result: Vec<Task> = Vec::new();

        let u = Url::parse(BASE_URL).unwrap();
        let mut cursor: Option<String> = None;

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
                .add_param("query", filter_to_query(project_name, f).as_str());

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

    pub async fn project(&self, id: &str) -> Result<Project, Box<dyn std::error::Error>> {
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
