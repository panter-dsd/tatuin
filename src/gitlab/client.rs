// SPDX-License-Identifier: MIT

use super::structs::{Issue, Todo};
use crate::filter::FilterState;
use reqwest::header::HeaderMap;
use serde::Serialize;
use std::error::Error;

#[derive(Serialize, Debug)]
pub struct UpdateIssueRequest<'a> {
    pub due_date: Option<&'a str>,
}

pub struct Client {
    base_url: String,
    default_header: HeaderMap,
    client: reqwest::Client,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitLab client base_url={}", self.base_url)
    }
}

impl Client {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {api_key}").parse().unwrap());
        Self {
            base_url: format!("{base_url}/api/v4"),
            default_header: headers,
            client: reqwest::Client::new(),
        }
    }

    #[tracing::instrument(level = "info", target = "gitlab_client")]
    pub async fn todos(&self, state: &FilterState) -> Result<Vec<Todo>, Box<dyn Error>> {
        let mut result = Vec::new();

        const PER_PAGE: i8 = 100;
        let mut page = 1;

        let state_query = match state {
            FilterState::Completed => "state=done".to_string(),
            FilterState::Uncompleted => "state=pending".to_string(),
            _ => return Ok(Vec::new()),
        };

        loop {
            let r = self
                .client
                .get(format!(
                    "{}/todos?page={page}&per_page={PER_PAGE}&{state_query}",
                    self.base_url
                ))
                .headers(self.default_header.clone())
                .send()
                .await?
                .error_for_status()?
                .json::<Vec<Todo>>()
                .await;

            match r {
                Ok(mut v) => {
                    if v.is_empty() {
                        break;
                    }
                    result.append(&mut v);
                    page += 1;
                }
                Err(e) => {
                    tracing::error!(target:"gitlab_todo_client", state_query=state_query, page=page, error=?e);
                    return Err(e.into());
                }
            }
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

    pub async fn project_issues_by_iids(&self, project_id: i64, iids: &[i64]) -> Result<Vec<Issue>, Box<dyn Error>> {
        let mut result = Vec::new();
        if iids.is_empty() {
            return Ok(result);
        }

        let query = iids
            .iter()
            .map(|iid| format!("iids[]={iid}"))
            .collect::<Vec<_>>()
            .join("&");
        tracing::debug!(target:"gitlab_todo_client", query=?query, project_id=project_id);

        const PER_PAGE: i8 = 100;
        let mut page = 1;

        loop {
            let r = self
                .client
                .get(format!(
                    "{}/projects/{project_id}/issues?page={page}&per_page={PER_PAGE}&scope=all&{query}",
                    self.base_url
                ))
                .headers(self.default_header.clone())
                .send()
                .await?
                .json::<Vec<Issue>>()
                .await;

            match r {
                Ok(mut v) => {
                    if v.is_empty() {
                        break;
                    }

                    result.append(&mut v);
                    page += 1;
                }
                Err(e) => {
                    tracing::error!(target:"gitlab_todo_client", query=?query, page=page, error=?e);
                    return Err(e.into());
                }
            }
        }

        Ok(result)
    }

    pub async fn patch_issue(
        &self,
        project_id: i64,
        issue_iid: i64,
        r: &UpdateIssueRequest<'_>,
    ) -> Result<(), Box<dyn Error>> {
        tracing::debug!(target:"gitlab_client", project_id=project_id, issue_iid=issue_iid, request=?r, "Patch issue");

        self
            .client
            .put(format!("{}/projects/{project_id}/issues/{issue_iid}", self.base_url))
            .json(r)
            .headers(self.default_header.clone())
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"gitlab_client", project_id=project_id, issue_iid=issue_iid, request=?r, error=?e);
                Box::<dyn Error>::from(e.to_string())
            })
    }
}
