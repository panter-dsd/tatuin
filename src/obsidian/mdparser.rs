use crate::obsidian::task::{State, Task};
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

    fn tasks_from_content(&self, content: String) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let mut result: Vec<Task> = Vec::new();

        const SPLIT_TERMINATOR: &str = "\n";

        let mut pos: u64 = 0;

        for l in content.split(SPLIT_TERMINATOR) {
            if let Some(caps) = TASK_RE.captures(l) {
                let t = Task {
                    file_path: String::from(self.file_path.as_str()),
                    pos,
                    state: {
                        let cap: &str = &caps[1];
                        match cap.chars().next() {
                            Some(x) => State::new(x),
                            None => panic!(
                                "Something wronng with regexp parsing of '{l}' because state shouldn't be empty"
                            ),
                        }
                    },
                    text: String::from(&caps[2]),
                };
                result.push(t);
            }
            pos += (l.len() + SPLIT_TERMINATOR.len()) as u64;
        }

        Ok(result)
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
    fn parse_empty_content() {
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
}
