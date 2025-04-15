use crate::obsidian::task::{State, Task};
use crate::task::DateTimeUtc;
use chrono::{NaiveDate, Utc};
use regex::Regex;
use std::fs;
use std::sync::LazyLock;

static TASK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\ *-\ \[(.)\]\ (.*)$").unwrap());

pub struct Parser {
    file_path: String,
}

impl Parser {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: String::from(file_path),
        }
    }

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(self.file_path.as_str())?;
        self.tasks_from_content(content)
    }

    fn try_parse_task(&self, line: &str, pos: u64) -> Option<Task> {
        if let Some(caps) = TASK_RE.captures(line) {
            let task_text = String::from(&caps[2]);
            return Some(Task {
                root_path: String::new(),
                file_path: self.file_path.to_string(),
                pos,
                state: {
                    let cap: &str = &caps[1];
                    match cap.chars().next() {
                        Some(x) => State::new(x),
                        None => panic!(
                            "Something wronng with regexp parsing of '{line}' because state shouldn't be empty"
                        ),
                    }
                },
                text: task_text.to_string(),
                due: try_parse_due(task_text.as_str()),
            });
        }

        None
    }

    fn tasks_from_content(&self, content: String) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut result: Vec<Task> = Vec::new();

        const SPLIT_TERMINATOR: &str = "\n";

        let mut pos: u64 = 0;

        for l in content.split(SPLIT_TERMINATOR) {
            if let Some(t) = self.try_parse_task(l, pos) {
                let tt = t;
                result.push(tt);
            }

            pos += (l.len() + SPLIT_TERMINATOR.len()) as u64;
        }

        Ok(result)
    }
}

fn try_parse_due(text: &str) -> Option<DateTimeUtc> {
    const PATTERN: &str = "📅 ";
    let idx = text.rfind(PATTERN)?;
    println!("HERE {}", &text[idx + PATTERN.len()..]);

    match NaiveDate::parse_from_str(&text[idx + PATTERN.len()..], "%Y-%m-%d") {
        Ok(d) => {
            let dt = d.and_hms_opt(0, 0, 0)?;
            Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc))
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn parse_not_exists_file() {
        let p = Parser::new("/etc/file/not/exists");
        let err = p.tasks().await.unwrap_err();
        if let Some(error) = err.downcast_ref::<std::io::Error>() {
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        } else {
            panic!("Expected an IoError, but got a different error.");
        }
    }

    #[test]
    fn parse_content() {
        struct Case<'a> {
            name: &'a str,
            file_content: &'a str,
            count: usize,
        }
        const CASES: &[Case] = &[
            Case {
                name: "emty content",
                file_content: "",
                count: 0,
            },
            Case {
                name: "non emty content without tasks",
                file_content: "some text",
                count: 0,
            },
            Case {
                name: "content contain the single task and nothing else",
                file_content: "- [ ] Some text",
                count: 1,
            },
            Case {
                name: "content contain the single task and other text",
                file_content: "some text
- [ ] Some text
some another text
",
                count: 1,
            },
            Case {
                name: "several tasks",
                file_content: "some text
- [ ] Correct task
     - [ ] Correct task
- [x] Correct task
- [/] Correct task
-- [ ] Wrong task
- [] Wrong task
- [aa] Wrong task
- [ ]
-[ ] Wrong task
some another text
",
                count: 4,
            },
        ];

        let p = Parser::new("");

        for c in CASES {
            let tasks = p.tasks_from_content(String::from(c.file_content)).unwrap();
            assert_eq!(tasks.len(), c.count, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    fn parse_due() {
        struct Case<'a> {
            name: &'a str,
            line: &'a str,
            expected: Option<DateTimeUtc>,
        }
        let cases: &[Case] = &[
            Case {
                name: "empty string",
                line: "",
                expected: None,
            },
            Case {
                name: "correct string",
                line: "Some text ⏫ 📅 2025-01-27",
                expected: Some(DateTimeUtc::from_naive_utc_and_offset(
                    NaiveDate::parse_from_str("2025-01-27", "%Y-%m-%d")
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap(),
                    Utc,
                )),
            },
        ];

        for c in cases {
            let dt = try_parse_due(c.line);
            assert_eq!(dt, c.expected, "Test {} was failed", c.name);
        }
    }
}
