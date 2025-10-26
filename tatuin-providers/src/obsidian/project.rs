// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};

use tatuin_core::project::Project as ProjectTrait;

use crate::obsidian::utils;

#[derive(Clone)]
pub struct Project {
    provider: String,
    vault_path: PathBuf,
    file_path: PathBuf,
}

impl Project {
    pub fn new(provider: &str, vault_path: &Path, file_path: &Path) -> Self {
        Self {
            provider: provider.to_string(),
            vault_path: vault_path.into(),
            file_path: file_path.into(),
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
        utils::strip_root_str(&self.vault_path, &self.file_path)
    }

    fn name(&self) -> String {
        self.file_path
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
