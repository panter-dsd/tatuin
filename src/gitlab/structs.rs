// SPDX-License-Identifier: MIT

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub project: Option<Project>,
    pub author: Option<Author>,
    pub action_name: Option<String>,
    pub target_type: String,
    pub target: Option<Target>,
    pub target_url: String,
    pub body: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub name_with_namespace: Option<String>,
    pub path: String,
    pub path_with_namespace: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub name: Option<String>,
    pub username: Option<String>,
    pub id: Option<i64>,
    pub state: Option<String>,
    pub avatar_url: Option<String>,
    pub web_url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub id: i64,
    pub iid: i64,
    pub project_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub state: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub target_branch: Option<String>,
    pub source_branch: Option<String>,
    pub upvotes: Option<i64>,
    pub downvotes: Option<i64>,
    pub author: Option<Author>,
    pub assignee: Option<Assignee>,
    pub source_project_id: Option<i64>,
    pub target_project_id: Option<i64>,
    pub labels: Option<Vec<String>>,
    pub draft: Option<bool>,
    pub work_in_progress: Option<bool>,
    pub milestone: Option<Milestone>,
    pub merge_when_pipeline_succeeds: Option<bool>,
    pub merge_status: Option<String>,
    pub user_notes_count: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignee {
    pub name: Option<String>,
    pub username: Option<String>,
    pub id: Option<i64>,
    pub state: Option<String>,
    pub avatar_url: Option<String>,
    pub web_url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub id: Option<i64>,
    pub iid: Option<i64>,
    pub project_id: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub state: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub state: Option<String>,
    pub description: Option<String>,
    pub id: i64,
    pub title: Option<String>,
    pub created_at: String,
    pub iid: i64,
    pub due_date: Option<String>,
    pub issue_type: String,
}
