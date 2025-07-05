// SPDX-License-Identifier: MIT

use crate::filter::FilterState;

use super::structs::Issue;
use reqwest::{Method, RequestBuilder, header::HeaderMap};
use std::error::Error;

pub struct Client {
    base_url: String,
    default_header: HeaderMap,
    client: reqwest::Client,
}

impl Client {
    pub fn new(api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {api_key}").parse().unwrap());
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert("User-Agent", "Tatuin".parse().unwrap());
        Self {
            base_url: "https://api.github.com".to_string(),
            default_header: headers,
            client: reqwest::Client::new(),
        }
    }

    fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client.request(method, url).headers(self.default_header.clone())
    }

    pub async fn issues(&self, repo: &str, states: &[FilterState]) -> Result<Vec<Issue>, Box<dyn Error>> {
        let mut result = Vec::new();

        const PER_PAGE: i8 = 100;
        let mut page = 1;

        let state_query = if states.len() == 1 {
            match states[0] {
                FilterState::Completed => "state=closed".to_string(),
                FilterState::Uncompleted => "state=open".to_string(),
                _ => return Err(Box::<dyn Error>::from(format!("wrong state {}", states[0]))),
            }
        } else {
            "state=all".to_string()
        };

        loop {
            let url = format!(
                "{}/repos/{repo}/issues?page={page}&per_page={PER_PAGE}&{state_query}",
                self.base_url
            );
            match self.request(Method::GET, &url).send().await?.json::<Vec<Issue>>().await {
                Ok(mut r) => {
                    if r.is_empty() {
                        break;
                    }

                    result.append(&mut r);
                    page += 1;
                }
                Err(e) => {
                    tracing::error!(target:"github_client", url=url, error=?e);
                    return Err(e.into());
                }
            }
        }

        Ok(result)
    }
}
