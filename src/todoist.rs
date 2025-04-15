use reqwest::header::HeaderMap;
use serde::Deserialize;

const BASE_URL: &str = "https://todoist.com/api/v1";

pub struct Task {}

pub struct Project {
    pub id: String,
    pub name: String,
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

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let tasks: Vec<Task> = Vec::new();
        let resp = self
            .client
            .get(format!("{BASE_URL}/tasks"))
            .headers(self.default_header.clone())
            .send()
            .await?
            .json::<get_tasks::Response>()
            .await?;
        println!("{resp:#?}");
        Ok(tasks)
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

mod get_tasks {
    use super::*;

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Duration {
        property1: String,
        property2: String,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Task {
        id: String,
        user_id: String,
        project_id: String,
        section_id: Option<String>,
        parent_id: Option<String>,
        added_by_uid: Option<String>,
        assigned_by_uid: Option<String>,
        responsible_uid: Option<String>,
        labels: Vec<String>,
        deadline: Option<Duration>,
        duration: Option<Duration>,
        checked: bool,
        is_deleted: bool,
        added_at: Option<String>,
        completed_at: Option<String>,
        updated_at: Option<String>,
        // due: ???,
        priority: i32,
        child_order: i32,
        content: String,
        description: String,
        note_count: i32,
        day_order: i32,
        is_collapsed: bool,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct Response {
        results: Vec<Task>,
        next_cursor: String,
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
