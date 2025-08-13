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
            file_name: crate::folders::temp_folder().join(FILE_NAME),
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
        let buf = BufReader::new(File::open(&self.file_name)?);
        let reader = IcalParser::new(buf);

        read_tasks_from_calendar(reader)
    }
}

fn read_tasks_from_calendar<B>(reader: IcalParser<B>) -> Result<Vec<Task>, Box<dyn Error>>
where
    B: std::io::BufRead,
{
    let mut tasks = Vec::new();
    for line in reader {
        let calendar = line?;
        tasks.append(&mut calendar.events.iter().map(event_to_task).collect::<Vec<Task>>());
        tasks.append(&mut calendar.todos.iter().map(todo_to_task).collect::<Vec<Task>>());
    }
    Ok(Vec::new())
}
fn event_to_task(ev: &IcalEvent) -> Task {
    println!("{ev:?}");
    let mut t = Task::default();

    for p in &ev.properties {
        match p.name.as_str() {
            "SUMMARY" => t.name = p.value.clone().unwrap_or_default(),
            "UID" => t.uid = p.value.clone().unwrap_or_default(),
            _ => {}
        }
    }
    todo!("Not implemented")
}
fn todo_to_task(todo: &IcalTodo) -> Task {
    todo!("Not implemented")
}

mod test {
    use super::*;

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
SUMMARY:Have a shave
UID:12657849-3238754386-0@todoist.com
DTSTART;VALUE=DATE:20250814
DTEND;VALUE=DATE:20250815
DESCRIPTION:Project: Daily\n\nComplete this task: \nhttps://app.todoist.com/app/task/662FwG65MFXv2M3f?
END:VEVENT
END:VCALENDAR
";

    #[test]
    fn event_to_task_test() {
        let buf = BufReader::with_capacity(CALENDAR.len(), CALENDAR);
        let reader = IcalParser::new(buf);
        let tasks = read_tasks_from_calendar(reader);
        assert!(tasks.is_ok());

        let tasks = tasks.unwrap();
        assert!(!tasks.is_empty());
    }
}
