// SPDX-License-Identifier: MIT

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use regex::Regex;
use tatuin_core::RichStringTrait;

use crate::obsidian::{fs, markdown, md_file::TAG_RE};

static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+://[^\s$)]+").unwrap());

#[derive(Debug, Clone, Default)]
pub struct InternalLinksRenderer {
    raw: String,
    display: String,

    vault_path: Option<PathBuf>,
}

impl PartialEq for InternalLinksRenderer {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for InternalLinksRenderer {}

impl InternalLinksRenderer {
    pub fn new(s: &str) -> Self {
        Self {
            raw: s.to_string(),
            display: s.to_string(),
            vault_path: None,
        }
    }

    pub fn remove_tags(&mut self) {
        self.display = clear_tags(&self.display);
    }

    pub fn set_vault_path(&mut self, p: &Path) {
        if self.vault_path.is_none() {
            self.display = fix_regular_links(self.display.as_str(), p);
            self.display = fix_wiki_links(self.display.as_str(), p);
            self.display = fix_raw_links(self.display.as_str());
            self.vault_path = Some(p.to_path_buf());
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

fn fix_raw_links(text: &str) -> String {
    let mut result = String::new();

    let mut last_end: usize = 0;

    for m in URL_RE.find_iter(text) {
        let start = m.start();
        if start > 3 /*^[](http://) case*/ && text.get(start-2..start).is_some_and(|s| s == "](") {
            // detect []() link
            continue;
        }
        let mut s = m.as_str();
        let mut end = m.end();
        // check correctness of the url because the regexp is very simple
        if url::Url::parse(s).is_err() {
            if url::Url::parse(&s[..s.len() - 1]).is_ok() {
                end -= 1;
                s = &s[..s.len() - 1];
            } else {
                continue;
            }
        }

        result.push_str(&text[last_end..start]);
        result.push('[');
        result.push_str(s);
        result.push_str("](");
        result.push_str(s);
        result.push(')');
        last_end = end;
    }
    result.push_str(&text[last_end..]);
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

#[cfg(test)]
mod test {
    use super::fix_raw_links;

    #[test]
    fn fix_raw_links_test() {
        const FIXTURE:&str = "
obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81
[link](obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81)
Some http://some another http://hello/: and one more (http://yeeeee)
- [ ] Some task http://yandex.ru/some/uri 📅 2025-09-18
    First line https://login:password@host:1243/path/inside/file.txt: here
    Second line [link](http://ya.ru)
    Third line
";
        const RESULT:&str = "
[obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81](obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81)
[link](obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81)
Some [http://some](http://some) another [http://hello/:](http://hello/:) and one more ([http://yeeeee](http://yeeeee))
- [ ] Some task [http://yandex.ru/some/uri](http://yandex.ru/some/uri) 📅 2025-09-18
    First line [https://login:password@host:1243/path/inside/file.txt:](https://login:password@host:1243/path/inside/file.txt:) here
    Second line [link](http://ya.ru)
    Third line
";

        let t = fix_raw_links(FIXTURE);
        assert_eq!(RESULT, t);
    }
}
