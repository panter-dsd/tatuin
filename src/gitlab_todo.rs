// SPDX-License-Identifier: MIT

use crate::filter;
use crate::gitlab::client::Client;
use crate::gitlab::structs;
use crate::project::Project as ProjectTrait;
use crate::provider::{PatchError, Provider as ProviderTrait, TaskPatch};
use crate::task::{DateTimeUtc, State, Task as TaskTrait};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ratatui::style::Color;
use std::any::Any;
use std::error::Error;

use async_trait::async_trait;

pub const PROVIDER_NAME: &str = "GitLabTODO";

#[derive(Clone)]
pub struct Project {
    p: structs::Project,
    provider: String,
}

impl ProjectTrait for Project {
    fn id(&self) -> String {
        self.p.id.to_string()
    }

    fn name(&self) -> String {
        self.p.name.to_string()
    }

    fn provider(&self) -> String {
        self.provider.to_string()
    }

    fn description(&self) -> String {
        self.p.path.to_string()
    }

    fn parent_id(&self) -> Option<String> {
        None
    }

    fn is_inbox(&self) -> bool {
        false
    }

    fn is_favorite(&self) -> bool {
        false
    }

    fn clone_boxed(&self) -> Box<dyn ProjectTrait> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct Task {
    todo: structs::Todo,
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
        self.todo.id.to_string()
    }

    fn text(&self) -> String {
        self.todo.body.to_string()
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        str_to_date(self.todo.created_at.as_str())
    }

    fn due(&self) -> Option<DateTimeUtc> {
        self.created_at()
            .map(|dt| dt.with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()).unwrap())
    }

    fn place(&self) -> String {
        self.todo.target_url.to_string()
    }

    fn state(&self) -> State {
        match self.todo.state.as_str() {
            "pending" => State::Uncompleted,
            "done" => State::Completed,
            _ => panic!("Unknown state {}", self.todo.state),
        }
    }

    fn provider(&self) -> String {
        self.provider.to_string()
    }

    fn project(&self) -> Option<Box<dyn ProjectTrait>> {
        Some(Box::new(Project {
            p: self.todo.project.clone(),
            provider: self.provider.to_string(),
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }
}

pub struct Provider {
    name: String,
    color: Color,
    client: Client,
    tasks: Vec<Task>,
    last_filter: Option<filter::Filter>,
}

impl Provider {
    pub fn new(name: &str, base_url: &str, api_key: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            color: *color,
            client: Client::new(base_url, api_key),
            tasks: Vec::new(),
            last_filter: None,
        }
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
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn Error>> {
        let mut should_clear = false;
        if let Some(last_filter) = self.last_filter.as_mut() {
            should_clear = last_filter != f;
        }

        if should_clear {
            self.tasks.clear();
        }

        if self.tasks.is_empty() {
            for st in &f.states {
                for t in self.client.todos(st).await? {
                    self.tasks.push(Task {
                        todo: t,
                        provider: self.name(),
                    })
                }
            }
        }

        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();

        for t in &self.tasks {
            result.push(Box::new(t.clone()));
        }

        self.last_filter = Some(f.clone());

        Ok(result)
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>> {
        Ok(Vec::new())
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut errors = Vec::new();

        for p in patches {
            if let Some(state) = &p.state {
                match state {
                    State::Completed => match self.client.mark_todo_as_done(p.task.id().as_str()).await {
                        Ok(_) => self.tasks.clear(),
                        Err(e) => errors.push(PatchError {
                            task: p.task.clone_boxed(),
                            error: e.to_string(),
                        }),
                    },
                    State::InProgress | State::Uncompleted | State::Unknown(_) => {
                        errors.push(PatchError {
                            task: p.task.clone_boxed(),
                            error: format!("The state {state} is unsupported"),
                        });
                    }
                }
            }
        }

        errors
    }

    async fn reload(&mut self) {
        self.tasks.clear();
    }

    fn color(&self) -> Color {
        self.color
    }
}
