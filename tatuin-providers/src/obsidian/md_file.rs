// SPDX-License-Identifier: MIT

use crate::obsidian::{description::Description, indent, state::State, task::Task};
use chrono::{NaiveDate, Utc};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::{error::Error, path::PathBuf};
use tatuin_core::{
    RichStringTrait,
    task::{DateTimeUtc, Priority},
    task_patch::ValuePatch,
};

use super::patch::TaskPatch;

static TASK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*-\ \[(.)\]\ (.*)$").unwrap());
pub(crate) static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"( #((?:[^\x00-\x7F]|\w)(?:[^\x00-\x7F]|\w|-|_|\/)+))").unwrap());

const DUE_EMOJI: char = '📅';
const COMPLETED_EMOJI: char = '✅';

pub struct File {
    file_path: PathBuf,
    content: String,
}

impl File {
    pub fn new(file_path: &Path) -> Self {
        Self {
            file_path: file_path.into(),
            content: String::new(),
        }
    }

    pub fn open(&mut self) -> Result<(), std::io::Error> {
        self.content = fs::read_to_string(&self.file_path)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        if let Err(err) = fs::write(&self.file_path, &self.content) {
            return Err(Box::new(err));
        }

        Ok(())
    }

    pub async fn tasks(&self) -> Result<Vec<Task>, Box<dyn Error>> {
        self.tasks_from_content(&self.content)
    }

    pub async fn patch_task(&mut self, p: &TaskPatch<'_>) -> Result<(), Box<dyn Error>> {
        self.content = self.patch_task_in_content(p, self.content.as_str())?;
        Ok(())
    }

    pub async fn delete_task(&mut self, t: &Task) -> Result<(), Box<dyn Error>> {
        self.content = self.delete_task_from_content(t, self.content.as_str())?;
        Ok(())
    }

    fn try_parse_task(&self, line: &str, pos: usize) -> Option<Task> {
        let caps = TASK_RE.captures(line)?;

        let text = String::from(&caps[2]);
        let (text, due) = extract_date_after_emoji(text.as_str(), DUE_EMOJI);
        let (text, completed_at) = extract_date_after_emoji(text.as_str(), COMPLETED_EMOJI);
        let (text, priority) = extract_priority(text.as_str());

        let tags = TAG_RE
            .captures_iter(text.clone().as_str())
            .map(|tag_cap| tag_cap[2].to_string())
            .collect::<Vec<String>>();

        Some(Task {
            file_path: self.file_path.clone(),
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
            name: text.trim().into(),
            due,
            priority,
            completed_at,
            tags,
            ..Default::default()
        })
    }

    fn tasks_from_content(&self, content: &str) -> Result<Vec<Task>, Box<dyn Error>> {
        const SPLIT_TERMINATOR: &str = "\n";

        let mut result: Vec<Task> = Vec::new();

        let mut pos: usize = 0;

        let mut task: Option<Task> = None;

        for l in content.split(SPLIT_TERMINATOR) {
            if let Some(t) = self.try_parse_task(l, pos) {
                if let Some(previous_task) = task {
                    result.push(previous_task);
                }
                task = Some(t);
            } else if let Some(t) = &mut task {
                if indent::exists(l) {
                    t.description = Some(t.description.clone().unwrap_or(Description::new(pos)).append(l));
                } else {
                    result.push(t.clone());
                    task = None;
                }
            }

            pos += l.chars().count() + SPLIT_TERMINATOR.len();
        }

        if let Some(t) = task {
            result.push(t);
        }

        Ok(result)
    }

    fn check_task_was_not_changed(&self, t: &Task, content: &str) -> Result<(), Box<dyn Error>> {
        let line = content
            .chars()
            .skip(t.start_pos)
            .take(t.end_pos - t.start_pos)
            .collect::<String>();

        match self.try_parse_task(&line, t.start_pos) {
            Some(mut task) => {
                if let Some(d) = &t.description {
                    task.description = Some(Description::from_content(content, d.start, d.end));
                }
                if &task != t {
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

        Ok(())
    }

    fn patch_task_in_content(&self, p: &TaskPatch, content: &str) -> Result<String, Box<dyn Error>> {
        let current_task = p.task;
        self.check_task_was_not_changed(current_task, content)?;

        let mut new_task = p.task.clone();

        if let ValuePatch::Value(n) = &p.name {
            new_task.name = n.into();
        }

        if p.description.is_set() {
            new_task.description = p.description.value().map(|t| Description::from_str(t.as_str()));
        }

        if let ValuePatch::Value(v) = p.state {
            new_task.completed_at = (v == State::Completed).then_some(chrono::Utc::now());
            new_task.state = v;
        }

        if let ValuePatch::Value(v) = p.priority {
            new_task.priority = v;
        }

        if p.due.is_set() {
            new_task.due = p.due.value();
        }

        let indent = content
            .chars()
            .skip(current_task.start_pos)
            .take_while(indent::is_indent)
            .collect::<String>();
        Ok([
            content.chars().take(current_task.start_pos).collect::<String>(),
            indent.clone(),
            task_to_string(&new_task, indent.as_str()),
            content
                .chars()
                .skip(
                    current_task
                        .description
                        .as_ref()
                        .map(|d| d.end)
                        .unwrap_or(current_task.end_pos),
                )
                .collect::<String>(),
        ]
        .join(""))
    }

    fn delete_task_from_content(&self, t: &Task, content: &str) -> Result<String, Box<dyn Error>> {
        self.check_task_was_not_changed(t, content)?;
        Ok([
            content.chars().take(t.start_pos).collect::<String>(),
            content.chars().skip(t.end_pos + 1).collect::<String>(),
        ]
        .join(""))
    }
}

pub fn task_to_string(t: &Task, indent: &str) -> String {
    let state_char: char = t.state.into();
    let mut elements = vec![format!("- [{state_char}]"), t.name.raw()];
    if let Some(due) = &t.due {
        elements.push(format!("{DUE_EMOJI} {}", due.format("%Y-%m-%d")))
    }
    let priority_str = priority_to_str(&t.priority).to_string();
    if !priority_str.is_empty() {
        elements.push(priority_str);
    }
    if let Some(d) = &t.completed_at {
        elements.push(format!("{COMPLETED_EMOJI} {}", d.format("%Y-%m-%d")))
    }
    let mut s = elements.join(" ");
    if let Some(d) = &t.description {
        for l in d.text.split("\n") {
            s.push_str(format!("\n{indent}    ").as_str());
            s.push_str(l);
        }
    }

    s
}

fn extract_date_after_emoji(text: &str, emoji: char) -> (String, Option<DateTimeUtc>) {
    let start = format!(" {emoji} ");
    let idx = text.rfind(start.as_str());
    if idx.is_none() {
        return (text.to_string(), None);
    }

    let idx = idx.unwrap();

    const DATE_PATTERN: &str = "0000-00-00";

    if idx + start.len() + DATE_PATTERN.len() > text.len() {
        // wrong date
        return (text.to_string(), None);
    }

    let date_str = &text[idx + start.len()..idx + start.len() + DATE_PATTERN.len()];

    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        && let Some(dt) = d.and_hms_opt(0, 0, 0)
    {
        return (
            [
                text[..idx].to_string(),
                text[idx + start.len() + DATE_PATTERN.len()..].to_string(),
            ]
            .join(""),
            Some(DateTimeUtc::from_naive_utc_and_offset(dt, Utc)),
        );
    }

    (text.to_string(), None)
}

const PRIORITY_CHARS: [char; 5] = ['⏬', '🔽', '🔼', '⏫', '🔺'];
const fn char_to_priority(c: char) -> Priority {
    match c {
        '⏬' => Priority::Lowest,
        '🔽' => Priority::Low,
        '🔼' => Priority::Medium,
        '⏫' => Priority::High,
        '🔺' => Priority::Highest,
        _ => Priority::Normal,
    }
}
const fn priority_to_str(p: &Priority) -> &str {
    match p {
        Priority::Lowest => "⏬",
        Priority::Low => "🔽",
        Priority::Medium => "🔼",
        Priority::High => "⏫",
        Priority::Highest => "🔺",
        Priority::Normal => "",
    }
}

fn extract_priority(text: &str) -> (String, Priority) {
    let mut symbol_indexes = Vec::new();
    for s in PRIORITY_CHARS {
        if let Some(idx) = text.chars().position(|c| c == s)
            && idx != 0
            && text.chars().nth(idx - 1).unwrap_or(' ') == ' '
            && idx != text.len() - 1
            && text.chars().nth(idx + 1).unwrap_or(' ') == ' '
        {
            symbol_indexes.push((char_to_priority(s), idx));
        }
    }

    if symbol_indexes.is_empty() {
        return (text.to_string(), Priority::Normal);
    }

    symbol_indexes.sort_by_key(|x| x.1);
    let last = &symbol_indexes[symbol_indexes.len() - 1];

    let mut result_text = text.chars().take(last.1 - 1).collect::<String>();
    result_text.push_str(text.to_string().chars().skip(last.1 + 1).collect::<String>().as_str());

    (result_text, last.0)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn parse_not_exists_file_test() {
        let mut p = File::new(Path::new("/etc/file/not/exists"));
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

        let p = File::new(Path::new("/"));

        for c in CASES {
            let tasks = p.tasks_from_content(c.file_content).unwrap();
            assert_eq!(tasks.len(), c.count, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    fn check_all_fields_parsed_test() {
        let text = format!(
            "- [x] Some #tag task #группа/имя_tag-name123 text ⏫ {DUE_EMOJI} 2025-01-01 {COMPLETED_EMOJI} 2025-01-01 #tag_at_end"
        );

        let p = File::new(Path::new(""));
        let task = p.try_parse_task(text.as_str(), 0);
        assert!(task.is_some());
        let mut task = task.unwrap();
        task.set_vault_path(Path::new("."));
        assert_eq!(
            task.name.raw(),
            "Some #tag task #группа/имя_tag-name123 text #tag_at_end"
        );
        assert_eq!(task.name.display(), "Some task text");
        assert_eq!(task.state, State::Completed);
        assert!(task.due.is_some());
        assert_eq!(task.due.unwrap().format("%Y-%m-%d").to_string(), "2025-01-01");
        assert!(task.completed_at.is_some());
        assert_eq!(task.completed_at.unwrap().format("%Y-%m-%d").to_string(), "2025-01-01");
        assert_eq!(task.tags, vec!["tag", "группа/имя_tag-name123", "tag_at_end"]);
    }

    #[test]
    fn parse_description_test() {
        let text = "Some content
- [ ] Some task
 Description
  different indent
 return indent back
    tabulation as indent
 return indent back
End of content
";

        let p = File::new(Path::new(""));
        let tasks = p.tasks_from_content(text).unwrap();
        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        assert_eq!(task.name.raw(), "Some task");
        assert!(task.description.is_some());
        assert_eq!(
            task.description.as_ref().unwrap().text,
            "Description
different indent
return indent back
tabulation as indent
return indent back"
        );
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
            Case {
                name: "broken date",
                line: "Some text ⏫ 📅 2025-",
                expected: None,
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
            file_content_before: String,
            file_content_after: String,
        }
        let cases: &[Case] = &[
            Case {
                name: "content contain the single task and nothing else",
                file_content_before: "- [ ] Some text".to_string(),
                file_content_after: format!("- [x] Some text{completed_string}"),
            },
            Case {
                name: "content contain the single task and other text",
                file_content_before: "some text
- [ ] Some text
some another text
"
                .to_string(),
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
"
                .to_string(),
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
"
                .to_string(),
                file_content_after: format!(
                    "какой-то текст
- [x] Текст задачи{completed_string}
длинный текст в конце
"
                ),
            },
            Case {
                name: "content contain priority and due",
                file_content_before: format!(
                    "some text
- [ ] Correct task ⏫ {DUE_EMOJI} 2025-03-01
some text
"
                ),
                file_content_after: format!(
                    "some text
- [x] Correct task {DUE_EMOJI} 2025-03-01 ⏫{completed_string}
some text
"
                ),
            },
            Case {
                name: "content contain due and priority",
                file_content_before: format!(
                    "some text
- [ ] Correct task {DUE_EMOJI} 2025-03-01 ⏫
some text
"
                ),
                file_content_after: format!(
                    "some text
- [x] Correct task {DUE_EMOJI} 2025-03-01 ⏫{completed_string}
some text
"
                ),
            },
        ];

        let p = File::new(Path::new(""));

        for c in cases {
            let original_tasks = p.tasks_from_content(c.file_content_before.as_str()).unwrap();
            let mut tasks = original_tasks.clone();
            let mut result = c.file_content_before.to_string();
            for i in 0..original_tasks.len() {
                let r = p.patch_task_in_content(
                    &TaskPatch {
                        task: &tasks[i],
                        name: ValuePatch::NotSet,
                        description: ValuePatch::NotSet,
                        state: ValuePatch::Value(State::Completed),
                        due: ValuePatch::NotSet,
                        priority: ValuePatch::NotSet,
                    },
                    result.as_str(),
                );
                assert!(r.is_ok(), "{}: {}", c.name, r.unwrap_err());
                result = r.unwrap();
                tasks = p.tasks_from_content(&result).unwrap();
            }
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
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

        let p = File::new(Path::new(""));

        for c in CASES {
            let original_tasks = p.tasks_from_content(c.file_content_before).unwrap();
            let mut tasks = original_tasks.clone();
            let mut result = c.file_content_before.to_string();
            for i in 0..original_tasks.len() {
                let r = p.patch_task_in_content(
                    &TaskPatch {
                        name: ValuePatch::NotSet,
                        description: ValuePatch::NotSet,
                        task: &tasks[i],
                        state: ValuePatch::Value(State::Uncompleted),
                        due: ValuePatch::NotSet,
                        priority: ValuePatch::NotSet,
                    },
                    result.as_str(),
                );
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
        let tasks = File::new(Path::new("")).tasks_from_content(content);
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
        let tasks = File::new(Path::new("")).tasks_from_content(content);
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
            let (s, p) = extract_priority(c.line);
            assert_eq!(p, c.expected_priority, "Test {} was failed", c.name);
            assert_eq!(s, c.expected_string, "Test {} was failed", c.name);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn change_priority_in_content_test() {
        struct Case<'a> {
            name: &'a str,
            file_content_before: String,
            file_content_after: String,
            priority: Priority,
        }
        let cases: &[Case] = &[
            Case {
                name: "normal to low",
                file_content_before: "- [ ] Some text".to_string(),
                file_content_after: "- [ ] Some text 🔽".to_string(),
                priority: Priority::Low,
            },
            Case {
                name: "low to normal",
                file_content_before: "- [ ] Some text 🔽".to_string(),
                file_content_after: "- [ ] Some text".to_string(),
                priority: Priority::Normal,
            },
            Case {
                name: "low to high",
                file_content_before: "- [ ] Some text 🔽".to_string(),
                file_content_after: "- [ ] Some text ⏫".to_string(),
                priority: Priority::High,
            },
            Case {
                name: "due and normal to low",
                file_content_before: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01"),
                file_content_after: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01 🔽"),
                priority: Priority::Low,
            },
            Case {
                name: "due and low to normal",
                file_content_before: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01 🔽"),
                file_content_after: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01"),
                priority: Priority::Normal,
            },
            Case {
                name: "due and low to high",
                file_content_before: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01 🔽"),
                file_content_after: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01 ⏫"),
                priority: Priority::High,
            },
            Case {
                name: "due and low to high priority before due",
                file_content_before: format!("- [ ] Some text 🔽 {DUE_EMOJI} 2025-03-01"),
                file_content_after: format!("- [ ] Some text {DUE_EMOJI} 2025-03-01 ⏫"),
                priority: Priority::High,
            },
        ];

        let p = File::new(Path::new(""));

        for c in cases {
            let original_tasks = p.tasks_from_content(c.file_content_before.as_str()).unwrap();
            let mut tasks = original_tasks.clone();
            let mut result = c.file_content_before.to_string();
            for i in 0..original_tasks.len() {
                let r = p.patch_task_in_content(
                    &TaskPatch {
                        task: &tasks[i],
                        name: ValuePatch::NotSet,
                        description: ValuePatch::NotSet,
                        state: ValuePatch::NotSet,
                        due: ValuePatch::NotSet,
                        priority: ValuePatch::Value(c.priority),
                    },
                    result.as_str(),
                );
                assert!(r.is_ok(), "{}: {}", c.name, r.unwrap_err());
                result = r.unwrap();
                tasks = p.tasks_from_content(&result).unwrap();
            }
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn task_patching_test() {
        struct Case<'a> {
            name: &'a str,
            file_content_before: String,
            file_content_after: String,
            patch: TaskPatch<'a>,
        }
        let cases: &[Case] = &[
            Case {
                name: "unchanged",
                file_content_before: format!("  - [ ] Some text #tag #другой.тег {DUE_EMOJI} 2025-03-01 ⏫ #tag3"),
                file_content_after: format!("  - [ ] Some text #tag #другой.тег #tag3 {DUE_EMOJI} 2025-03-01 ⏫"),
                patch: TaskPatch {
                    task: &Task::default(),
                    name: ValuePatch::NotSet,
                    description: ValuePatch::NotSet,
                    state: ValuePatch::NotSet,
                    due: ValuePatch::NotSet,
                    priority: ValuePatch::NotSet,
                },
            },
            Case {
                name: "change name with tabulation as indent",
                file_content_before: format!("\t- [ ] Some text #tag #другой.тег {DUE_EMOJI} 2025-03-01 ⏫ #tag3"),
                file_content_after: format!("\t- [ ] Some another text {DUE_EMOJI} 2025-03-01 ⏫"),
                patch: TaskPatch {
                    task: &Task::default(),
                    name: ValuePatch::Value("Some another text".to_string()),
                    description: ValuePatch::NotSet,
                    state: ValuePatch::NotSet,
                    due: ValuePatch::NotSet,
                    priority: ValuePatch::NotSet,
                },
            },
            Case {
                name: "change all",
                file_content_before: format!(
                    "  - [ ] Some text #tag #другой.тег {DUE_EMOJI} 2025-03-01 ⏫ #tag3
  task description
  on two lines"
                ),
                file_content_after: format!(
                    "  - [/] Some another text {DUE_EMOJI} 2025-01-27 🔺
      the task description"
                ),
                patch: TaskPatch {
                    task: &Task::default(),
                    name: ValuePatch::Value("Some another text".to_string()),
                    description: ValuePatch::Value("the task description".to_string()),
                    state: ValuePatch::Value(State::InProgress),
                    due: ValuePatch::Value(DateTimeUtc::from_naive_utc_and_offset(
                        NaiveDate::parse_from_str("2025-01-27", "%Y-%m-%d")
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap(),
                        Utc,
                    )),
                    priority: ValuePatch::Value(Priority::Highest),
                },
            },
            Case {
                name: "change description",
                file_content_before: "
- [ ] One two three 📅 2025-09-18
      текст
      some another text
- [ ] another task 📅 2025-09-18
"
                .to_string(),
                file_content_after: "
- [ ] One two three 📅 2025-09-18
    измененный текст из одной строки
    второй строки
    и третьей тоже
- [ ] another task 📅 2025-09-18
"
                .to_string(),
                patch: TaskPatch {
                    task: &Task::default(),
                    name: ValuePatch::NotSet,
                    description: ValuePatch::Value(
                        "измененный текст из одной строки
второй строки
и третьей тоже"
                            .to_string(),
                    ),
                    state: ValuePatch::NotSet,
                    due: ValuePatch::NotSet,
                    priority: ValuePatch::NotSet,
                },
            },
            Case {
                name: "change description in task with indentation",
                file_content_before: "
  - [ ] One two three 📅 2025-09-18
      текст
      some another text
- [ ] another task 📅 2025-09-18
"
                .to_string(),
                file_content_after: "
  - [ ] One two three 📅 2025-09-18
      измененный текст из одной строки
      второй строки
      и третьей тоже
- [ ] another task 📅 2025-09-18
"
                .to_string(),
                patch: TaskPatch {
                    task: &Task::default(),
                    name: ValuePatch::NotSet,
                    description: ValuePatch::Value(
                        "измененный текст из одной строки
второй строки
и третьей тоже"
                            .to_string(),
                    ),
                    state: ValuePatch::NotSet,
                    due: ValuePatch::NotSet,
                    priority: ValuePatch::NotSet,
                },
            },
        ];

        let p = File::new(Path::new(""));

        for c in cases {
            let original_tasks = p.tasks_from_content(c.file_content_before.as_str()).unwrap();
            let task = original_tasks[0].clone();
            let mut result = c.file_content_before.to_string();
            let patch = TaskPatch {
                task: &task,
                ..c.patch.clone()
            };
            let r = p.patch_task_in_content(&patch, result.as_str());
            assert!(r.is_ok(), "{}: {}", c.name, r.unwrap_err());
            result = r.unwrap();
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn task_deleting_test() {
        struct Case<'a> {
            name: &'a str,
            file_content_before: &'a str,
            file_content_after: &'a str,
            task_number: usize,
        }
        let cases: &[Case] = &[
            Case {
                name: "single task without any content",
                file_content_before: "- [ ] Some text #tag #другой.тег 2025-03-01 ⏫ #tag3",
                file_content_after: "",
                task_number: 0,
            },
            Case {
                name: "single task with content",
                file_content_before: "Some content
- [ ] Some text #tag #другой.тег 2025-03-01 ⏫ #tag3
Some another content
",
                file_content_after: "Some content
Some another content
",
                task_number: 0,
            },
            Case {
                name: "single task with content and leader spaces",
                file_content_before: "Some content
       - [ ] Some text #tag #другой.тег 2025-03-01 ⏫ #tag3
Some another content
",
                file_content_after: "Some content
Some another content
",
                task_number: 0,
            },
            Case {
                name: "several tasks with content",
                file_content_before: "Some content
- [ ] Some text #tag #другой.тег 2025-03-01 ⏫ #tag3
- [ ] Some another text #tag #другой.тег 2025-03-01 ⏫ #tag3
Some another content
",
                file_content_after: "Some content
- [ ] Some text #tag #другой.тег 2025-03-01 ⏫ #tag3
Some another content
",
                task_number: 1,
            },
        ];

        let p = File::new(Path::new(""));

        for c in cases {
            let tasks = p.tasks_from_content(c.file_content_before).unwrap();
            assert!(tasks.len() > c.task_number);
            let task = tasks[c.task_number].clone();
            let mut result = c.file_content_before.to_string();
            let r = p.delete_task_from_content(&task, result.as_str());
            assert!(r.is_ok(), "{}: {}", c.name, r.unwrap_err());
            result = r.unwrap();
            assert_eq!(c.file_content_after, result, "Test '{}' was failed", c.name);
        }
    }
}
