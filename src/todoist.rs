use reqwest::header::HeaderMap;
use serde::Deserialize;

const BASE_URL: &str = "https://todoist.com/api/v1";

pub struct Task {}

pub struct Todoist {
    default_header: HeaderMap,
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
        }
    }

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let tasks: Vec<Task> = Vec::new();
        let resp = reqwest::Client::new()
            .get(format!("{BASE_URL}/tasks"))
            .headers(self.default_header.clone())
            .send()
            .await?
            .json::<get_tasks::Response>()
            .await?;
        println!("{resp:#?}");
        Ok(tasks)
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
