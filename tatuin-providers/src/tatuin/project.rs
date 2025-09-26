use redb::Value;
use serde::{Deserialize, Serialize};
use tatuin_core::project::Project as ProjectTrait;

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Project {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub parent: Option<uuid::Uuid>,
    pub is_inbox: bool,

    #[serde(skip_serializing, skip_deserializing)]
    provider_name: String,
}

pub fn inbox_project(provider_name: &str) -> Project {
    Project {
        id: uuid::Uuid::new_v4(),
        name: "Inbox".to_string(),
        description: "Inbox project".to_string(),
        parent: None,
        is_inbox: true,
        provider_name: provider_name.to_string(),
    }
}

impl Project {
    pub fn set_provider_name(&mut self, name: &str) {
        self.provider_name = name.to_string()
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
        self.provider_name.clone()
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

impl Value for Project {
    type SelfType<'a>
        = Project
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap_or_default()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Vec<u8>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap_or_default()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Project")
    }
}
