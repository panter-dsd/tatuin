// SPDX-License-Identifier: MIT

use std::{fmt::Display, path::Path};

use tatuin_core::RichStringTrait;

use crate::obsidian::{fs, markdown, md_file::TAG_RE};

#[derive(Debug, Clone, Default)]
pub struct InternalLinksRenderer {
    raw: String,
    display: String,

    vault_path_was_set: bool,
}

impl PartialEq for InternalLinksRenderer {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for InternalLinksRenderer {}

impl InternalLinksRenderer {
    fn new(s: &str) -> Self {
        Self {
            raw: s.to_string(),
            display: s.to_string(),
            vault_path_was_set: false,
        }
    }

    pub fn remove_tags(&mut self) {
        self.display = clear_tags(&self.display);
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
            // there is no existed file, so render as-is
            continue;
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

impl<T> From<T> for InternalLinksRenderer
where
    T: Display,
{
    fn from(value: T) -> Self {
        Self::new(value.to_string().as_str())
    }
}

impl RichStringTrait for InternalLinksRenderer {
    fn raw(&self) -> String {
        self.raw.clone()
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}
