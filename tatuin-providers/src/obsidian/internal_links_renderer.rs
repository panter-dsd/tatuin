// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};

use tatuin_core::RichStringTransformerTrait;

use crate::obsidian::{fs, markdown, md_file::TAG_RE};

#[derive(Debug, Clone, Default)]
pub struct InternalLinksTransformer {
    vault_path: PathBuf,
    remove_tags: bool,
}

impl InternalLinksTransformer {
    pub fn new(vault_path: &Path) -> Self {
        Self {
            vault_path: vault_path.to_path_buf(),
            remove_tags: false,
        }
    }

    pub fn with_remove_tags(mut self) -> Self {
        self.remove_tags = true;
        self
    }
}

impl RichStringTransformerTrait for InternalLinksTransformer {
    fn transform(&self, s: &str) -> String {
        let mut s = s.to_string();

        if self.remove_tags {
            s = clear_tags(&s);
        }
        s = fix_regular_links(&s, &self.vault_path);
        s = fix_wiki_links(&s, &self.vault_path);
        s
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

fn fix_regular_links(text: &str, vault_path: &Path) -> String {
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
