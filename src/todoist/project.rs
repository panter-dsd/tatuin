use serde::Deserialize;

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
