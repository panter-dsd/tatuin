// SPDX-License-Identifier: MIT

use crate::{
    filter,
    github::{client::Client, structs},
    project::Project as ProjectTrait,
    provider::{GetTasksError, ProviderTrait},
    task::{DateTimeUtc, PatchPolicy, State, Task as TaskTrait, due_group},
    task_patch::{PatchError, TaskPatch},
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ratatui::style::Color;
use std::{any::Any, error::Error};

use async_trait::async_trait;

pub const PROVIDER_NAME: &str = "GitHub Issues";

#[derive(Clone)]
pub struct Task {
    issue: structs::Issue,
    provider: String,
}

fn str_to_date(s: &str) -> Option<DateTimeUtc> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0)?;
        return Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc));
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc));
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(DateTimeUtc::from(dt));
    }

    None
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        self.issue.id.to_string()
    }

    fn text(&self) -> String {
        self.issue.title.to_string()
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        str_to_date(self.issue.created_at.as_str())
    }

    fn due(&self) -> Option<DateTimeUtc> {
        if let Some(m) = &self.issue.milestone {
            if let Some(due) = &m.due_on {
                return str_to_date(due.as_str());
            }
        }

        None
    }

    fn place(&self) -> String {
        self.issue.html_url.to_string()
    }

    fn state(&self) -> State {
        match self.issue.state.as_str() {
            "open" => State::Uncompleted,
            "closed" => State::Completed,
            _ => panic!("Unknown state {}", self.issue.state),
        }
    }

    fn provider(&self) -> String {
        self.provider.to_string()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn url(&self) -> String {
        self.issue.html_url.to_string()
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            available_states: Vec::new(),
            available_priorities: Vec::new(),
            available_due_items: Vec::new(),
        }
    }
}

pub struct Provider {
    name: String,
    color: Color,
    repo: String,
    client: Client,
    tasks: Vec<Task>,
    last_filter: Option<filter::Filter>,
}

impl Provider {
    pub fn new(name: &str, api_key: &str, repo: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            color: *color,
            repo: repo.to_string(),
            client: Client::new(api_key),
            tasks: Vec::new(),
            last_filter: None,
        }
    }
}

impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider name={}", ProviderTrait::name(self))
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, GetTasksError> {
        let mut should_clear = false;
        if let Some(last_filter) = self.last_filter.as_mut() {
            should_clear = last_filter != f;
        }

        if should_clear {
            self.tasks.clear();
        }

        if self.tasks.is_empty() {
            for t in self.client.issues(&self.repo, &f.states).await? {
                self.tasks.push(Task {
                    issue: t,
                    provider: self.name(),
                })
            }
        }

        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();

        for t in &self.tasks {
            if f.due.contains(&due_group(t)) {
                result.push(Box::new(t.clone()));
            }
        }

        self.last_filter = Some(f.clone());

        Ok(result)
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>> {
        Ok(Vec::new())
    }

    async fn patch_tasks(&mut self, _patches: &[TaskPatch]) -> Vec<PatchError> {
        Vec::new()
    }

    async fn reload(&mut self) {
        self.tasks.clear();
    }

    fn color(&self) -> Color {
        self.color
    }
}
