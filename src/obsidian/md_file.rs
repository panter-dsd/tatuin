// SPDX-License-Identifier: MIT

use crate::obsidian::task::{State, Task};
use crate::task::{DateTimeUtc, Priority};
use chrono::{NaiveDate, Utc};
use regex::Regex;
use std::error::Error;
use std::fs;
use std::sync::LazyLock;

static TASK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*-\ \[(.)\]\ (.*)$").unwrap());
const DUE_EMOJI: char = '📅';
const COMPLETED_EMOJI: char = '✅';

pub struct File {
    file_path: String,
    content: String,
}

impl File {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: String::from(file_path),
            content: String::new(),
        }
    }

    pub fn open(&mut self) -> Result<(), std::io::Error> {
        self.content = fs::read_to_string(self.file_path.as_str())?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        if let Err(err) = fs::write(self.file_path.as_str(), &self.content) {
            return Err(Box::new(err));
        }

        Ok(())
    }

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn Error>> {
        self.tasks_from_content(&self.content)
    }

    pub async fn change_state(&mut self, t: &Task, s: State) -> Result<(), Box<dyn Error>> {
        self.content = self.change_state_in_content(t, s, self.content.as_str())?;
        Ok(())
    }

    fn try_parse_task(&self, line: &str, pos: usize) -> Option<Task> {
        if let Some(caps) = TASK_RE.captures(line) {
            let text = String::from(&caps[2]);
            let (text, due) = extract_date_after_emoji(text.as_str(), DUE_EMOJI);
            let (text, completed_at) = extract_date_after_emoji(text.as_str(), COMPLETED_EMOJI);
            let (text, priority) = parse_priority(text.as_str());
            return Some(Task {
                file_path: self.file_path.to_string(),
                start_pos: pos,
                end_pos: pos + line.chars().count(),
                state: {
                    let cap: &str = &caps[1];
                    match cap.chars().next() {
                        Some(x) => State::new(x),
                        None => {
                            panic!("Something wrong with regexp parsing of '{line}' because state shouldn't be empty")
                        }
                    }
                },
                text: text.trim().to_string(),
                due,
                priority,
                completed_at,
                ..Default::default()
            });
        }

        None
    }

    fn tasks_from_content(&self, content: &str) -> Result<Vec<Task>, Box<dyn Error>> {
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

    fn change_state_in_content(&self, t: &Task, s: State, content: &str) -> Result<String, Box<dyn Error>> {
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
        let result: String = content
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

        if s == State::Completed {
            Ok([
                result.chars().take(t.end_pos).collect::<String>(),
                format!(" {COMPLETED_EMOJI} {}", chrono::Utc::now().format("%Y-%m-%d")),
                result.chars().skip(t.end_pos).collect::<String>(),
            ]
            .join(""))
        } else {
            let task: String = result.chars().skip(t.start_pos).take(t.end_pos - t.start_pos).collect();
            let (task, _) = extract_date_after_emoji(task.as_str(), COMPLETED_EMOJI);

            Ok([
                result.chars().take(t.start_pos).collect::<String>(),
                task,
                result.chars().skip(t.end_pos).collect::<String>(),
            ]
            .join(""))
        }
    }
}

fn extract_date_after_emoji(text: &str, emoji: char) -> (String, Option<DateTimeUtc>) {
    let start = format!(" {emoji} ");
    let idx = text.rfind(start.as_str());
    if idx.is_none() {
        return (text.to_string(), None);
    }

    let idx = idx.unwrap();

    const DATE_PATTERN: &str = "0000-00-00";

    let date_str = &text[idx + start.len()..idx + start.len() + DATE_PATTERN.len()];

    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        if let Some(dt) = d.and_hms_opt(0, 0, 0) {
            return (
                [
                    text[..idx].to_string(),
                    text[idx + start.len() + DATE_PATTERN.len()..].to_string(),
                ]
                .join(""),
                Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc)),
            );
        }
    }

    (text.to_string(), None)
}

fn parse_priority(text: &str) -> (String, Priority) {
    let symbols = vec![
        ('⏬', Priority::Lowest),
        ('🔽', Priority::Low),
        ('🔼', Priority::Medium),
        ('⏫', Priority::High),
        ('🔺', Priority::Highest),
    ];
    let mut symbol_indexes = Vec::new();
    for (s, p) in symbols {
        if let Some(idx) = text.chars().position(|c| c == s) {
            if idx != 0
                && text.chars().nth(idx - 1).unwrap_or(' ') == ' '
                && idx != text.len() - 1
                && text.chars().nth(idx + 1).unwrap_or(' ') == ' '
            {
                symbol_indexes.push((p, idx));
            }
        }
    }

    if symbol_indexes.is_empty() {
        return (text.to_string(), Priority::Normal);
    }

    symbol_indexes.sort_by_key(|x| x.1);
    let last = &symbol_indexes[symbol_indexes.len() - 1];

    let mut result_text = text.chars().take(last.1 - 1).collect::<String>();
    result_text.push_str(text.to_string().chars().skip(last.1 + 1).collect::<String>().as_str());

    (result_text, last.0.clone())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn parse_not_exists_file_test() {
        let mut p = File::new("/etc/file/not/exists");
        let err = p.open().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn parse_content_test() {
        struct Case<'a> {
            name: &'a str,
            file_content: &'a str,
            count: usize,
        }
        const CASES: &[Case] = &[
            Case {
                name: "empty content",
                file_content: "",
                count: 0,
            },
            Case {
                name: "non empty content without tasks",
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
                name: "content contain cyrillic",
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
                count: 5,
            },
        ];

        let p = File::new("");

        for c in CASES {
            let tasks = p.tasks_from_content(c.file_content).unwrap();
            assert_eq!(tasks.len(), c.count, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    fn check_all_fields_parsed_test() {
        let text = format!("- [x] Some text ⏫ {DUE_EMOJI} 2025-01-01 {COMPLETED_EMOJI} 2025-01-01");

        let p = File::new("");
        let task = p.try_parse_task(text.as_str(), 0);
        assert!(task.is_some());
        let task = task.unwrap();
        assert_eq!(task.text, "Some text");
        assert_eq!(task.state, State::Completed);
        assert!(task.due.is_some());
        assert_eq!(task.due.unwrap().format("%Y-%m-%d").to_string(), "2025-01-01");
        assert!(task.completed_at.is_some());
        assert_eq!(task.completed_at.unwrap().format("%Y-%m-%d").to_string(), "2025-01-01");
    }

    #[test]
    fn parse_due_test() {
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
            let (_, dt) = extract_date_after_emoji(c.line, DUE_EMOJI);
            assert_eq!(dt, c.expected, "Test {} was failed", c.name);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn change_state_to_complete_in_content_test() {
        let completed_string = format!(" ✅ {}", chrono::Utc::now().format("%Y-%m-%d"));
        struct Case<'a> {
            name: &'a str,
            file_content_before: &'a str,
            file_content_after: String,
        }
        let cases: &[Case] = &[
            Case {
                name: "content contain the single task and nothing else",
                file_content_before: "- [ ] Some text",
                file_content_after: format!("- [x] Some text{completed_string}"),
            },
            Case {
                name: "content contain the single task and other text",
                file_content_before: "some text
- [ ] Some text
some another text
",
                file_content_after: format!(
                    "some text
- [x] Some text{completed_string}
some another text
"
                ),
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
                file_content_after: format!(
                    "some text
- [x] Correct task{completed_string}
     - [x] Correct task{completed_string}
- [x] Correct task{completed_string}
- [x] Correct task{completed_string}
-- [ ] Wrong task
- [] Wrong task
- [aa] Wrong task
- [ ]
-[ ] Wrong task
some another text
"
                ),
            },
            Case {
                name: "content contain cyrillic",
                file_content_before: "какой-то текст
- [ ] Текст задачи
длинный текст в конце
",
                file_content_after: format!(
                    "какой-то текст
- [x] Текст задачи{completed_string}
длинный текст в конце
"
                ),
            },
        ];

        let p = File::new("");

        for c in cases {
            let original_tasks = p.tasks_from_content(c.file_content_before).unwrap();
            let mut tasks = original_tasks.clone();
            let mut result = c.file_content_before.to_string();
            for i in 0..original_tasks.len() {
                let r = p.change_state_in_content(&tasks[i], State::Completed, result.as_str());
                assert!(r.is_ok(), "{}: {}", c.name, r.unwrap_err());
                result = r.unwrap();
                tasks = p.tasks_from_content(&result).unwrap();
            }
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    fn change_state_to_incomplete_in_content_test() {
        struct Case<'a> {
            name: &'a str,
            file_content_before: &'a str,
            file_content_after: &'a str,
        }
        const CASES: &[Case] = &[
            Case {
                name: "content contain the single task and nothing else",
                file_content_before: "- [x] Some text ✅ 2025-01-01",
                file_content_after: "- [ ] Some text",
            },
            Case {
                name: "content contain the single task without completed date",
                file_content_before: "- [x] Some text",
                file_content_after: "- [ ] Some text",
            },
            Case {
                name: "content contain the single task and other text",
                file_content_before: "some text
- [x] Some text ✅ 2025-01-01
some another text
",
                file_content_after: "some text
- [ ] Some text
some another text
",
            },
            Case {
                name: "several tasks",
                file_content_before: "some text
- [x] Correct task ✅ 2025-01-01
     - [x] Correct task ✅ 2025-01-01
- [x] Correct task ✅ 2025-01-01
- [/] Correct task
-- [ ] Wrong task
- [] Wrong task
- [aa] Wrong task
- [ ]
-[ ] Wrong task
some another text
",
                file_content_after: "some text
- [ ] Correct task
     - [ ] Correct task
- [ ] Correct task
- [ ] Correct task
-- [ ] Wrong task
- [] Wrong task
- [aa] Wrong task
- [ ]
-[ ] Wrong task
some another text
",
            },
            Case {
                name: "content contain cyrillic",
                file_content_before: "какой-то текст
- [x] Текст задачи ✅ 2025-01-01
длинный текст в конце
",
                file_content_after: "какой-то текст
- [ ] Текст задачи
длинный текст в конце
",
            },
        ];

        let p = File::new("");

        for c in CASES {
            let original_tasks = p.tasks_from_content(c.file_content_before).unwrap();
            let mut tasks = original_tasks.clone();
            let mut result = c.file_content_before.to_string();
            for i in 0..original_tasks.len() {
                let r = p.change_state_in_content(&tasks[i], State::Uncompleted, result.as_str());
                assert!(r.is_ok(), "{}: {}", c.name, r.unwrap_err());
                result = r.unwrap();
                tasks = p.tasks_from_content(&result).unwrap();
            }
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    fn test_pos_in_parse_content_for_for_eng_test() {
        let content = "Some text
- [ ] Task
Some another text";
        let tasks = File::new("").tasks_from_content(content);
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(1, tasks.len());
        assert_eq!(10, tasks[0].start_pos);
        assert_eq!(20, tasks[0].end_pos);
    }

    #[test]
    fn test_pos_in_parse_content_for_for_cyrillic_test() {
        let content = "Какой-то текст
- [ ] Задача
Какой-то другой текст";
        let tasks = File::new("").tasks_from_content(content);
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(1, tasks.len());
        assert_eq!(15, tasks[0].start_pos);
        assert_eq!(27, tasks[0].end_pos);
    }

    #[test]
    fn parse_priority_test() {
        struct Case<'a> {
            name: &'a str,
            line: &'a str,
            expected_string: &'a str,
            expected_priority: Priority,
        }
        let cases: &[Case] = &[
            Case {
                name: "empty string",
                line: "",
                expected_string: "",
                expected_priority: Priority::Normal,
            },
            Case {
                name: "correct string without priority",
                line: "Some text 📅 2025-01-27",
                expected_string: "Some text 📅 2025-01-27",
                expected_priority: Priority::Normal,
            },
            Case {
                name: "correct string with high priority",
                line: "Some text ⏫ 📅 2025-01-27",
                expected_string: "Some text 📅 2025-01-27",
                expected_priority: Priority::High,
            },
            Case {
                name: "correct string with low priority",
                line: "Some text 🔽 📅 2025-01-27",
                expected_string: "Some text 📅 2025-01-27",
                expected_priority: Priority::Low,
            },
            Case {
                name: "two different priorities",
                line: "Проверка ⏬задача 🔽 📅 2025-01-27",
                expected_string: "Проверка ⏬задача 📅 2025-01-27",
                expected_priority: Priority::Low,
            },
            Case {
                name: "string with the only priority without spaces",
                line: "🔽",
                expected_string: "🔽",
                expected_priority: Priority::Normal,
            },
            Case {
                name: "string with the only priority with surrounding spaces",
                line: " 🔽 ",
                expected_string: " ",
                expected_priority: Priority::Low,
            },
        ];

        for c in cases {
            let (s, p) = parse_priority(c.line);
            assert_eq!(p, c.expected_priority, "Test {} was failed", c.name);
            assert_eq!(s, c.expected_string, "Test {} was failed", c.name);
        }
    }
}
