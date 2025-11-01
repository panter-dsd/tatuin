// SPDX-License-Identifier: MIT

use super::indent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Description {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

impl Description {
    pub fn new(start: usize) -> Self {
        Self {
            text: String::new(),
            start,
            end: start,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            text: s.to_string(),
            start: 0,
            end: s.chars().count(),
        }
    }

    pub fn from_content(s: &str, start: usize, end: usize) -> Self {
        let text = s
            .chars()
            .skip(start)
            .take(end - start)
            .collect::<String>()
            .split('\n')
            .map(indent::trim_str)
            .collect::<Vec<&str>>()
            .join("\n");
        Self { text, start, end }
    }

    pub fn append(&self, line: &str) -> Self {
        let mut count = line.chars().count();
        let line = indent::trim_str(line);
        let text = if self.text.is_empty() {
            line.to_string()
        } else {
            count += 1;
            self.text.clone() + "\n" + line
        };
        Self {
            text,
            start: self.start,
            end: self.end + count,
        }
    }
}
