use crate::obsidian::task::{State, Task};
use crate::task::DateTimeUtc;
use chrono::{NaiveDate, Utc};
use regex::Regex;
use std::error::Error;
use std::fs;
use std::sync::LazyLock;

static TASK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\ *-\ \[(.)\]\ (.*)$").unwrap());

pub struct File {
    file_path: String,
}

impl File {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: String::from(file_path),
        }
    }

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn Error>> {
        let content = fs::read_to_string(self.file_path.as_str())?;
        self.tasks_from_content(content)
    }

    pub async fn change_state(&self, t: &Task, s: State) -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(self.file_path.as_str())?;
        let content = self.change_state_in_content(t, s, content.as_str())?;
        if let Err(err) = fs::write(self.file_path.as_str(), content) {
            return Err(Box::new(err));
        }

        Ok(())
    }

    fn try_parse_task(&self, line: &str, pos: usize) -> Option<Task> {
        if let Some(caps) = TASK_RE.captures(line) {
            let task_text = String::from(&caps[2]);
            let (text, due) = parse_due(task_text.as_str());
            return Some(Task {
                file_path: self.file_path.to_string(),
                start_pos: pos,
                end_pos: pos + line.chars().count(),
                state: {
                    let cap: &str = &caps[1];
                    match cap.chars().next() {
                        Some(x) => State::new(x),
                        None => panic!(
                            "Something wrong with regexp parsing of '{line}' because state shouldn't be empty"
                        ),
                    }
                },
                text,
                due,
                ..Default::default()
            });
        }

        None
    }

    fn tasks_from_content(&self, content: String) -> Result<Vec<Task>, Box<dyn Error>> {
        const SPLIT_TERMINATOR: &str = "\n";

        let mut result: Vec<Task> = Vec::new();

        let mut pos: usize = 0;

        for l in content.split(SPLIT_TERMINATOR) {
            if let Some(t) = self.try_parse_task(l, pos) {
                result.push(t);
            }

            pos += l.chars().count() + SPLIT_TERMINATOR.len();
        }

        Ok(result)
    }

    fn change_state_in_content(
        &self,
        t: &Task,
        s: State,
        content: &str,
    ) -> Result<String, Box<dyn Error>> {
        let line = content
            .chars()
            .skip(t.start_pos)
            .take(t.end_pos - t.start_pos)
            .collect::<String>();

        match self.try_parse_task(&line, t.start_pos) {
            Some(task) => {
                if task != *t {
                    return Err(Box::<dyn std::error::Error>::from(
                        "Task has been changed since last loading",
                    ));
                }
            }
            None => {
                return Err(Box::<dyn std::error::Error>::from(
                    "Task disapeader from the file since last loading",
                ));
            }
        }

        let mut pos_found = false;
        let mut found = false;
        let result = content
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let mut result = c;
                if i > t.start_pos && !found {
                    if pos_found {
                        found = true;
                        result = char::from(s.clone());
                    } else {
                        pos_found = c == '[';
                    }
                }
                result
            })
            .collect();
        Ok(result)
    }
}

fn parse_due(text: &str) -> (String, Option<DateTimeUtc>) {
    const DUE_START: &str = " 📅 ";
    let idx = text.rfind(DUE_START);
    if idx.is_none() {
        return (text.to_string(), None);
    }

    let idx = idx.unwrap();

    const DUE_PATTERN: &str = "0000-00-00";

    let date_str = &text[idx + DUE_START.len()..idx + DUE_START.len() + DUE_PATTERN.len()];

    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        if let Some(dt) = d.and_hms_opt(0, 0, 0) {
            return (
                text[..idx].to_string(),
                Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc)),
            );
        }
    }

    (text.to_string(), None)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn parse_not_exists_file() {
        let p = File::new("/etc/file/not/exists");
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
                name: "content contain cyrilic",
                file_content: "какой-то текст
- [ ] Текст задачи
длинный текст в конце
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

        let p = File::new("");

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
            Case {
                name: "spaces after date",
                line: "Some text ⏫ 📅 2025-01-27  ",
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

    #[test]
    fn change_state_in_content() {
        struct Case<'a> {
            name: &'a str,
            file_content_before: &'a str,
            file_content_after: &'a str,
        }
        const CASES: &[Case] = &[
            Case {
                name: "content contain the single task and nothing else",
                file_content_before: "- [ ] Some text",
                file_content_after: "- [x] Some text",
            },
            Case {
                name: "content contain the single task and other text",
                file_content_before: "some text
- [ ] Some text
some another text
",
                file_content_after: "some text
- [x] Some text
some another text
",
            },
            Case {
                name: "several tasks",
                file_content_before: "some text
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
                file_content_after: "some text
- [x] Correct task
     - [x] Correct task
- [x] Correct task
- [x] Correct task
-- [ ] Wrong task
- [] Wrong task
- [aa] Wrong task
- [ ]
-[ ] Wrong task
some another text
",
            },
            Case {
                name: "content contain cyrilic",
                file_content_before: "какой-то текст
- [ ] Текст задачи
длинный текст в конце
",
                file_content_after: "какой-то текст
- [x] Текст задачи
длинный текст в конце
",
            },
        ];

        let p = File::new("");

        for c in CASES {
            let tasks = p
                .tasks_from_content(c.file_content_before.to_string())
                .unwrap();
            let mut result = c.file_content_before.to_string();
            for t in tasks {
                let r = p.change_state_in_content(&t, State::Completed, result.as_str());
                assert!(r.is_ok(), "{}", r.unwrap());
                result = r.unwrap();
            }
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    fn test_pos_in_parse_content_for_for_eng() {
        let content = "Some text
- [ ] Task
Some another text";
        let tasks = File::new("").tasks_from_content(content.to_string());
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(1, tasks.len());
        assert_eq!(10, tasks[0].start_pos);
        assert_eq!(20, tasks[0].end_pos);
    }

    #[test]
    fn test_pos_in_parse_content_for_for_cyrilic() {
        let content = "Какой-то текст
- [ ] Задача
Какой-то другой текст";
        let tasks = File::new("").tasks_from_content(content.to_string());
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(1, tasks.len());
        assert_eq!(15, tasks[0].start_pos);
        assert_eq!(27, tasks[0].end_pos);
    }
}
