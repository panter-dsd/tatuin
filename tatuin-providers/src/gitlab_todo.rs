// SPDX-License-Identifier: MIT

use crate::{
    config::Config,
    gitlab::{
        client::{Client, UpdateIssueRequest},
        structs,
    },
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use std::{any::Any, collections::HashMap, error::Error};
use tatuin_core::{
    RawTaskName, StringError, filter,
    project::Project as ProjectTrait,
    provider::{Capabilities, ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
    task::{DateTimeUtc, PatchPolicy, State, Task as TaskTrait, TaskNameProvider, due_group},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
};

use async_trait::async_trait;

pub const PROVIDER_NAME: &str = "GitLabTODO";

#[derive(Clone)]
pub struct Project {
    p: structs::Project,
    provider: String,
}

impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Project id={} name={}",
            ProjectTrait::id(self),
            ProjectTrait::name(self)
        )
    }
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
    issue: Option<structs::Issue>,
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

    fn name(&self) -> Box<dyn TaskNameProvider> {
        Box::new(RawTaskName::from(&self.todo.body))
    }

    fn created_at(&self) -> Option<DateTimeUtc> {
        str_to_date(self.todo.created_at.as_str())
    }

    fn due(&self) -> Option<DateTimeUtc> {
        let _entered = tracing::span!(tracing::Level::TRACE, "gitlab_todo_task").entered();

        if let Some(issue) = &self.issue {
            tracing::trace!(issue=?issue);
            if let Some(due) = &issue.due_date {
                tracing::trace!(due=?due);
                return str_to_date(due.as_str());
            }
        }

        tracing::trace!("get due from created_at");
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
        self.todo.project.as_ref().map(|p| {
            let result: Box<dyn ProjectTrait> = Box::new(Project {
                p: p.clone(),
                provider: self.provider.to_string(),
            });
            result
        })
    }

    fn url(&self) -> String {
        self.todo.target_url.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        Box::new(self.clone())
    }

    fn const_patch_policy(&self) -> PatchPolicy {
        PatchPolicy {
            is_editable: false,
            is_removable: false,
            available_states: vec![State::Uncompleted, State::Completed],
            available_priorities: Vec::new(),
            available_due_items: if self.issue.is_some() {
                DuePatchItem::values()
            } else {
                Vec::new()
            },
        }
    }
}

pub struct Provider {
    cfg: Config,
    client: Client,
    tasks: Vec<Task>,
    last_filter: Option<filter::Filter>,
}

impl Provider {
    pub fn new(cfg: Config, base_url: &str, api_key: &str) -> Self {
        Self {
            cfg,
            client: Client::new(base_url, api_key),
            tasks: Vec::new(),
            last_filter: None,
        }
    }

    async fn load_todos_issues(&mut self, todos: &[structs::Todo]) -> Result<Vec<structs::Issue>, Box<dyn Error>> {
        let mut project_iids: HashMap<i64, Vec<i64>> = HashMap::new();
        for t in todos {
            if (t.target_type == "Issue" || t.target_type == "MergeRequest")
                && let Some(target) = &t.target
            {
                match project_iids.get_mut(&target.project_id) {
                    Some(iids) => {
                        iids.push(target.iid);
                    }
                    None => {
                        project_iids.insert(target.project_id, vec![target.iid]);
                    }
                }
            }
        }

        let mut issues = Vec::new();

        for (project_id, iids) in project_iids {
            let mut iss = self.client.project_issues_by_iids(project_id, &iids).await?;
            issues.append(&mut iss);
        }

        Ok(issues)
    }

    async fn patch_task_state(&mut self, t: &Task, state: &State) -> Result<(), PatchError> {
        match state {
            State::Completed => self
                .client
                .mark_todo_as_done(t.id().as_str())
                .await
                .map_err(|e| PatchError {
                    task: t.clone_boxed(),
                    error: e.to_string(),
                }),
            State::InProgress | State::Uncompleted | State::Unknown(_) => Err(PatchError {
                task: t.clone_boxed(),
                error: format!("The state {state} is unsupported"),
            }),
        }
    }
    async fn patch_task_due(&mut self, t: &Task, due: &DuePatchItem) -> Result<(), PatchError> {
        let issue = t.issue.as_ref().ok_or(PatchError {
            task: t.clone_boxed(),
            error: "The task doesn't support due changing".to_string(),
        })?;
        let dt: Option<DateTimeUtc> = (*due).into();
        let d = dt.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();

        self.client
            .patch_issue(
                issue.project_id,
                issue.iid,
                &UpdateIssueRequest {
                    due_date: Some(d.as_str()),
                },
            )
            .await
            .map_err(|e| PatchError {
                task: t.clone_boxed(),
                error: e.to_string(),
            })
    }
}

impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider name={}", ProviderTrait::name(self))
    }
}

#[async_trait]
impl ProjectProviderTrait for Provider {
    async fn list(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, StringError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl TaskProviderTrait for Provider {
    async fn list(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, StringError> {
        let mut should_clear = false;
        if let Some(last_filter) = self.last_filter.as_mut() {
            should_clear = last_filter != f;
        }

        if should_clear {
            self.tasks.clear();
        }

        if self.tasks.is_empty() {
            for st in &f.states {
                let todos = self.client.todos(st).await?;
                let issues = self.load_todos_issues(&todos).await?;

                tracing::debug!(target:"gitlab_todo", issues=?issues, "Get Issues");
                for t in todos {
                    let id = t.target.as_ref().map(|t| t.id);
                    self.tasks.push(Task {
                        todo: t,
                        issue: if let Some(id) = id {
                            issues.iter().find(|issue| issue.id == id).cloned()
                        } else {
                            None
                        },
                        provider: self.name(),
                    })
                }
            }
        }

        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();

        for t in &self.tasks {
            if f.due.contains(&due_group(&t.due())) {
                result.push(Box::new(t.clone()));
            }
        }

        self.last_filter = Some(f.clone());

        Ok(result)
    }

    async fn create(&mut self, _project_id: &str, _tp: &TaskPatch) -> Result<(), StringError> {
        Err(StringError::new("Task creation is not supported"))
    }

    async fn update(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut errors = Vec::new();

        for p in patches {
            tracing::debug!(target:"gitlab_todo_patch_task", patch=p.to_string(), "Apply a patch");
            let task = p.task.as_ref().unwrap();

            let task = match task.as_any().downcast_ref::<Task>() {
                Some(t) => t,
                None => panic!("Wrong casting!"),
            };
            if let Some(state) = &p.state.value()
                && let Err(e) = self.patch_task_state(task, state).await
            {
                errors.push(e);
            }
            if let Some(due) = &p.due.value()
                && let Err(e) = self.patch_task_due(task, due).await
            {
                errors.push(e);
            }
        }

        self.tasks.clear();

        errors
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.cfg.name()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn reload(&mut self) {
        self.tasks.clear();
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { create_task: false }
    }
}
