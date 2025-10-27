use std::{fmt::Display, path::Path};

use tatuin_core::task::TaskNameProvider as TaskNameProviderTrait;

use crate::obsidian::{fs, markdown, md_file::TAG_RE};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct TaskNameProvider {
    name: String,
    display: String,

    vault_path_was_set: bool,
}

impl TaskNameProvider {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            display: clear_tags(name),
            vault_path_was_set: false,
        }
    }

    pub fn set_vault_path(&mut self, p: &Path) {
        self.display = fix_wiki_links(self.display.as_str(), p);
        self.vault_path_was_set = true;
    }
}

fn clear_tags(name: &str) -> String {
    TAG_RE.replace_all(name, "").to_string()
}

fn fix_wiki_links(text: &str, vault_path: &Path) -> String {
    let mut result = text.to_string();

    for l in markdown::find_wiki_links(text).iter().rev() {
        let file_name = format!("{}.md", l.link);
        let link = if let Ok(f) = fs::find_file(vault_path, &file_name) {
            fs::obsidian_url(vault_path, &f)
        } else {
            l.link.to_string()
        };
        result.replace_range(l.start..l.end + 1, format!("[{}]({})", l.display_text, link).as_str());
    }

    result
}

impl<T> From<T> for TaskNameProvider
where
    T: Display,
{
    fn from(value: T) -> Self {
        Self::new(value.to_string().as_str())
    }
}

impl TaskNameProviderTrait for TaskNameProvider {
    fn raw(&self) -> String {
        self.name.clone()
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}
