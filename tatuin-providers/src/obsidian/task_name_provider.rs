use std::{fmt::Display, path::Path};

use tatuin_core::task::TaskNameProvider as TaskNameProviderTrait;

use crate::obsidian::{fs, markdown, md_file::TAG_RE};

#[derive(Debug, Clone, Default)]
pub struct TaskNameProvider {
    name: String,
    display: String,

    vault_path_was_set: bool,
}

impl PartialEq for TaskNameProvider {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for TaskNameProvider {}

impl TaskNameProvider {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            display: clear_tags(name),
            vault_path_was_set: false,
        }
    }

    pub fn set_vault_path(&mut self, p: &Path) {
        if !self.vault_path_was_set {
            self.display = fix_refular_links(self.display.as_str(), p);
            self.display = fix_wiki_links(self.display.as_str(), p);
            self.vault_path_was_set = true;
        }
    }
}

fn clear_tags(name: &str) -> String {
    TAG_RE.replace_all(name, "").to_string()
}

fn fix_wiki_links(text: &str, vault_path: &Path) -> String {
    let mut result = text.to_string();

    for l in markdown::find_wiki_links(text).iter().rev() {
        let file_name = format!("{}.md", l.link);

        let link = if let Ok(file_name) = urlencoding::decode(&file_name)
            && let Ok(f) = fs::find_file(vault_path, &file_name)
        {
            fs::obsidian_url(vault_path, &f)
        } else {
            l.link.to_string()
        };

        let display = if l.display_text.is_empty() {
            l.link.to_string()
        } else {
            l.display_text.to_string()
        };

        result.replace_range(l.start..l.end + 1, format!("[{display}]({link})").as_str());
    }

    result
}

fn fix_refular_links(text: &str, vault_path: &Path) -> String {
    let mut result = text.to_string();

    for l in markdown::find_regular_links(text).iter().rev() {
        if let Ok(file_name) = urlencoding::decode(l.link)
            && let Ok(f) = fs::find_file(vault_path, &file_name)
        {
            result.replace_range(
                l.start..l.end,
                format!("[{}]({})", l.display_text, fs::obsidian_url(vault_path, &f)).as_str(),
            );
        }
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
