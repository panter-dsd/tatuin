// SPDX-License-Identifier: MIT

use crate::project::Project as ProjectTrait;

#[derive(Clone)]
pub struct Project {
    provider: String,
    root_path: String,
    file_path: String,
}

impl Project {
    pub fn new(provider: &str, root_path: &str, file_path: &str) -> Self {
        Self {
            provider: provider.to_string(),
            root_path: root_path.to_string(),
            file_path: file_path.to_string(),
        }
    }
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
        self.file_path
            .strip_prefix(self.root_path.as_str())
            .unwrap_or_default()
            .to_string()
    }

    fn name(&self) -> String {
        std::path::Path::new(&self.file_path)
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .strip_suffix(".md")
            .unwrap_or_default()
            .to_string()
    }

    fn provider(&self) -> String {
        self.provider.to_string()
    }

    fn description(&self) -> String {
        String::new()
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
