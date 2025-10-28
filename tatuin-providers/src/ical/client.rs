// SPDX-License-Identifier: MIT

use std::{
    error::Error,
    fs::File,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
};

use ical::{
    IcalParser,
    parser::ical::component::{IcalEvent, IcalTodo},
};

use crate::ical::task::TaskType;

use super::task::Task;

const FILE_NAME: &str = "calendar.ics";

pub struct Client {
    url: String,
    file_name: PathBuf,
}

impl Client {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            file_name: tatuin_core::folders::temp_folder().join(FILE_NAME),
        }
    }

    pub fn set_cache_folder(&mut self, p: &Path) {
        self.file_name = p.join(FILE_NAME).to_path_buf();
    }

    pub async fn download_calendar(&self) -> Result<(), Box<dyn Error>> {
        let response = reqwest::get(&self.url).await?;
        let mut file = std::fs::File::create(&self.file_name)?;
        let mut content = Cursor::new(response.bytes().await?);
        std::io::copy(&mut content, &mut file)?;
        Ok(())
    }

    pub async fn parse_calendar(&self) -> Result<Vec<Task>, Box<dyn Error>> {
        parse_calendar(&self.file_name).await
    }
}

pub async fn parse_calendar(file_path: &PathBuf) -> Result<Vec<Task>, Box<dyn Error>> {
    let buf = BufReader::new(File::open(file_path)?);
    let reader = IcalParser::new(buf);

    let job = tokio::spawn(async move { read_tasks_from_calendar(reader) });
    job.await.unwrap().map_err(|e| Box::new(e) as Box<dyn Error>)
}

fn read_tasks_from_calendar<B>(reader: IcalParser<B>) -> Result<Vec<Task>, ical::parser::ParserError>
where
    B: std::io::BufRead,
{
    let mut tasks = Vec::new();

    for line in reader {
        let calendar = line?;
        tasks.append(
            &mut calendar
                .events
                .iter()
                .map(event_to_task)
                .filter(|t| t.is_valid())
                .collect::<Vec<Task>>(),
        );
        tasks.append(
            &mut calendar
                .todos
                .iter()
                .map(todo_to_task)
                .filter(|t| t.is_valid())
                .collect::<Vec<Task>>(),
        );
    }

    Ok(tasks)
}

fn event_to_task(ev: &IcalEvent) -> Task {
    let mut t = Task::from(&ev.properties);
    t.task_type = TaskType::Event;
    t
}

fn todo_to_task(todo: &IcalTodo) -> Task {
    let mut t = Task::from(&todo.properties);
    t.task_type = TaskType::Todo;
    t
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ical::task::TaskStatus;
    use tatuin_core::task::{Priority, State, Task};

    #[test]
    fn event_to_task_test() {
        const CALENDAR: &[u8] = b"
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Apple Computer\\, Inc//iCal 1.5//EN
CALSCALE:GREGORIAN
UID:todoist-12657849-istlightgenerator
X-WR-CALNAME:Todoist
X-WR-TIMEZONE:Etc/UTC
X-PUBLISHED-TTL:PT10M
X-APPLE-CALENDAR-COLOR:#D34E3A
REFRESH-INTERVAL;VALUE=DURATION:PT10M
BEGIN:VEVENT
SUMMARY:Task name
UID:12657849-3238754386-000000
DTSTART;VALUE=DATE:20250814
DTEND;VALUE=DATE:20250815
DURATION:PT1H0M0S
DUE:20250814T100000Z
PRIORITY:3
DESCRIPTION:Project: Daily\n\nComplete this task: \nhttps://app.todoist.com/app/task/662FwG65MFXv2M3f?
END:VEVENT
BEGIN:VTODO
UID:20070313T123432Z-456553@example.com
DTSTAMP:20070313T123432Z
DUE;VALUE=DATE:20070501
SUMMARY:Submit Quebec Income Tax Return for 2006
CLASS:CONFIDENTIAL
CATEGORIES:FAMILY,FINANCE
STATUS:NEEDS-ACTION
END:VTODO
END:VCALENDAR
";

        let buf = BufReader::with_capacity(CALENDAR.len(), CALENDAR);
        let reader = IcalParser::new(buf);
        let tasks = read_tasks_from_calendar(reader);
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert_eq!(tasks.len(), 2);

        let task = &tasks[0];
        assert_eq!(task.id(), "12657849-3238754386-000000");
        assert_eq!(task.name().raw(), "Task name");
        assert_eq!(task.status, TaskStatus::Confirmed);
        assert_eq!(task.state(), State::Uncompleted);
        assert_eq!(task.priority(), Priority::High);

        assert!(task.due.is_some());
        assert_eq!(task.due.unwrap().to_string(), "2025-08-14 10:00:00 UTC");

        assert!(task.start.is_some());
        assert_eq!(task.start.unwrap().to_string(), "2025-08-14 00:00:00 UTC");

        assert!(task.end.is_some());
        assert_eq!(task.end.unwrap().to_string(), "2025-08-15 00:00:00 UTC");

        assert!(task.duration.is_some());
        assert_eq!(task.duration.unwrap().num_seconds(), 3600);

        let task = &tasks[1];
        assert_eq!(task.id(), "20070313T123432Z-456553@example.com");
        assert_eq!(task.name().raw(), "Submit Quebec Income Tax Return for 2006");
        assert_eq!(task.status, TaskStatus::NeedsAction);
        assert_eq!(task.state(), State::Uncompleted);
        assert_eq!(task.priority(), Priority::Normal);
        assert_eq!(task.labels(), vec!["FAMILY", "FINANCE"]);

        assert!(task.due.is_some());
        assert_eq!(task.due.unwrap().to_string(), "2007-05-01 00:00:00 UTC");

        assert!(task.start.is_none());
        assert!(task.end.is_none());
        assert!(task.duration.is_none());
    }
}
