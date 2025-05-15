use super::structs::Todo;
use crate::filter::FilterState;
use reqwest::header::HeaderMap;
use std::error::Error;

pub struct Client {
    base_url: String,
    default_header: HeaderMap,
    client: reqwest::Client,
}

impl Client {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
        Self {
            base_url: format!("{base_url}/api/v4"),
            default_header: headers,
            client: reqwest::Client::new(),
        }
    }

    pub async fn todos(&self, state: &FilterState) -> Result<Vec<Todo>, Box<dyn Error>> {
        let mut result = Vec::new();

        const PER_PAGE: i8 = 100;
        let mut page = 0;

        let state_query = match state {
            FilterState::Completed => "state=done".to_string(),
            FilterState::Uncompleted => "state=pending".to_string(),
            _ => return Ok(Vec::new()),
        };

        loop {
            let mut resp = self
                .client
                .get(format!(
                    "{}/todos?page={page}&per_page={PER_PAGE}&{state_query}",
                    self.base_url
                ))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<Vec<Todo>>()
                .await?;
            if resp.is_empty() {
                break;
            }

            result.append(&mut resp);
            page += 1;
        }

        Ok(result)
    }

    pub async fn mark_todo_as_done(&self, id: &str) -> Result<(), Box<dyn Error>> {
        let _ = self
            .client
            .post(format!("{}/todos/{id}/mark_as_done", self.base_url))
            .headers(self.default_header.clone())
            .send()
            .await?
            .json::<Todo>()
            .await?;
        Ok(())
    }
}
