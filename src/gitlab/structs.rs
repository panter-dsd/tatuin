use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub project: Project,
    pub author: Author,
    pub action_name: String,
    pub target_type: String,
    pub target: Target,
    pub target_url: String,
    pub body: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub name_with_namespace: String,
    pub path: String,
    pub path_with_namespace: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub username: String,
    pub id: i64,
    pub state: String,
    pub avatar_url: String,
    pub web_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub id: i64,
    pub iid: i64,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub target_branch: String,
    pub source_branch: String,
    pub upvotes: i64,
    pub downvotes: i64,
    pub author: Author,
    pub assignee: Option<Assignee>,
    pub source_project_id: Option<i64>,
    pub target_project_id: Option<i64>,
    pub labels: Vec<String>,
    pub draft: bool,
    pub work_in_progress: bool,
    pub milestone: Option<Milestone>,
    pub merge_when_pipeline_succeeds: bool,
    pub merge_status: String,
    pub user_notes_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignee {
    pub name: String,
    pub username: String,
    pub id: i64,
    pub state: String,
    pub avatar_url: String,
    pub web_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub id: i64,
    pub iid: i64,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub due_date: String,
}
