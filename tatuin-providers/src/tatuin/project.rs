use serde::{Deserialize, Serialize};
use tatuin_core::project::Project as ProjectTrait;

use super::PROVIDER_NAME;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Project {
    id: uuid::Uuid,
    name: String,
    description: String,
    parent: Option<uuid::Uuid>,
    is_inbox: bool,
}

pub fn inbox_project() -> Project {
    Project {
        id: uuid::Uuid::new_v4(),
        name: "Inbox".to_string(),
        description: "Inbox project".to_string(),
        parent: None,
        is_inbox: true,
    }
}

impl ProjectTrait for Project {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn provider(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn parent_id(&self) -> Option<String> {
        self.parent.map(|u| u.to_string())
    }

    fn is_inbox(&self) -> bool {
        self.is_inbox
    }

    fn is_favorite(&self) -> bool {
        false
    }

    fn clone_boxed(&self) -> Box<dyn ProjectTrait> {
        Box::new(self.clone())
    }
}
