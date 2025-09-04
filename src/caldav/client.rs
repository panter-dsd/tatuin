use chrono::TimeDelta;
use ical::property::Property;
use itertools::Itertools;
use reqwest::{
    Method, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use reqwest_dav::{Auth, Client as WebDavClient, ClientBuilder, Depth, list_cmd::ListEntity};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

use crate::{
    ical::{Task, property_to_str},
    provider::StringError,
    task::DateTimeUtc,
};

const INDEX_FILE_NAME: &str = "index.toml";
const DEFAULT_EVENT_DURATION: TimeDelta = TimeDelta::hours(1);

pub struct Config {
    pub url: String,
    pub login: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CachedFile {
    href: String,
    last_modified: DateTimeUtc,
    etag: Option<String>,
    file_name: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
struct CachedFiles {
    files: Vec<CachedFile>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
enum StorageType {
    Calendar,
    Todo,
}

pub struct Client {
    cfg: Config,
    storage_type: Option<StorageType>,
    c: Option<WebDavClient>,
    cache_folder: PathBuf,
}

impl Client {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            storage_type: None,
            c: None,
            cache_folder: crate::folders::temp_folder(),
        }
    }

    pub fn set_cache_folder(&mut self, p: &Path) {
        self.cache_folder = p.to_path_buf()
    }

    pub async fn download(&mut self) -> Result<(), Box<dyn Error>> {
        let url = url::Url::parse(&self.cfg.url)?;
        let mut current_cached_files = self.load_cached_files().await;
        let mut new_cached_files = CachedFiles::default();

        tracing::debug!(uri = url.path(), "Get file list");

        let c = self.client()?;
        let files = c.list(url.path(), Depth::Number(1)).await?;

        for f in files {
            if let ListEntity::File(f) = f {
                if let Some(pos) = current_cached_files.files.iter().position(|cf| cf.href == f.href) {
                    let cached_file = current_cached_files.files.remove(pos);
                    if cached_file.etag == f.tag && cached_file.last_modified == f.last_modified {
                        tracing::debug!(href = f.href, "The file wasn't changed");
                        new_cached_files.files.push(cached_file);
                        continue;
                    }

                    tracing::debug!(href = f.href, "The file was changed");
                }

                new_cached_files.files.push(CachedFile {
                    file_name: self.download_and_save_file(f.href.as_str()).await?,
                    href: f.href.to_string(),
                    last_modified: f.last_modified,
                    etag: f.tag,
                });
            }
        }

        self.clean_missed_files(&current_cached_files.files).await;
        self.save_cached_files(&new_cached_files).await?;

        Ok(())
    }

    pub async fn parse_calendars(&mut self) -> Result<Vec<Task>, Box<dyn Error>> {
        let mut result = Vec::new();

        for f in self.load_cached_files().await.files {
            let mut tasks = self.parse_calendar(&f.file_name).await?;
            tasks.iter_mut().for_each(|t| t.href = f.href.clone());
            result.append(&mut tasks);
        }

        if !result.is_empty() {
            self.storage_type = Some(if result[0].task_type == crate::ical::TaskType::Event {
                StorageType::Calendar
            } else {
                StorageType::Todo
            });
        }

        Ok(result)
    }

    pub async fn create_or_update(&mut self, t: &Task) -> Result<(), Box<dyn Error>> {
        if let Some(st) = self.storage_type.clone() {
            let r = self.send_create_or_update_request(&st, t).await?;
            let st = r.status();
            if st != StatusCode::CREATED {
                let response_text = r.text().await?;
                tracing::error!(target:"caldav_client", response_text=?response_text, "Send create or update request");
                return Err(StringError::new(format!("Wrong response status {st}").as_str()).into());
            } else {
                return r
                    .error_for_status()
                    .map(|_| ())
                    .map_err(|e| Box::new(e) as Box<dyn Error>);
            }
        }

        tracing::info!(target:"caldav_client", "Try to detect storage type");

        // try to detect storage type
        let mut r = self.send_create_or_update_request(&StorageType::Calendar, t).await?;

        if r.status() == StatusCode::FORBIDDEN {
            let response_text = r.text().await;
            tracing::error!(target:"caldav_client", text=?response_text, "Wrong storage type");
            // wrong storage type
            r = self.send_create_or_update_request(&StorageType::Calendar, t).await?;
            self.storage_type = Some(StorageType::Todo);
        } else {
            self.storage_type = Some(StorageType::Calendar);
        }

        tracing::info!(target:"caldav_client", storage_type=?self.storage_type.as_ref().unwrap(), "The storage type has been detected");

        let r = r.error_for_status();
        r.map(|_| ()).map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

impl Client {
    fn client(&mut self) -> Result<&WebDavClient, Box<dyn Error>> {
        if self.c.is_none() {
            let mut u = url::Url::parse(&self.cfg.url)?;
            u.set_path("/");
            tracing::debug!(
                url = u.to_string(),
                login = self.cfg.login,
                password = self.cfg.password,
                "Connect to the server"
            );
            self.c = Some(
                ClientBuilder::new()
                    .set_host(u.to_string())
                    .set_auth(Auth::Basic(self.cfg.login.clone(), self.cfg.password.clone()))
                    .build()?,
            );
        }

        Ok(self.c.as_ref().unwrap())
    }

    async fn load_cached_files(&self) -> CachedFiles {
        if let Ok(s) = tokio::fs::read_to_string(self.cache_folder.join(INDEX_FILE_NAME)).await {
            match toml::from_str(s.as_str()) {
                Ok(files) => return files,
                Err(e) => tracing::error!(error=?e, "Load cached files"),
            }
        }

        CachedFiles::default()
    }

    async fn save_cached_files(&self, files: &CachedFiles) -> Result<(), Box<dyn Error>> {
        let s = toml::to_string(files)?;
        tokio::fs::write(self.cache_folder.join(INDEX_FILE_NAME), s).await?;
        Ok(())
    }

    async fn clean_missed_files(&self, files: &[CachedFile]) {
        for f in files {
            tracing::debug!(href = f.href, file_name = f.file_name, "Remove cached file");
            if let Err(e) = tokio::fs::remove_file(self.cache_folder.join(f.file_name.as_str())).await {
                tracing::error!(error=?e, path=?self.cache_folder, file=f.file_name, "Remove cached file");
            }
        }
    }

    async fn download_and_save_file(&mut self, href: &str) -> Result<String, Box<dyn Error>> {
        let c = self.client()?;
        let mut r = c.get(href).await?;

        let file_name = file_name_from_href(href)?;
        tracing::debug!(href = href, file_name = file_name, "Download the file");

        let mut f = tokio::fs::File::create(self.cache_folder.join(file_name.as_str())).await?;
        while let Some(chunk) = r.chunk().await? {
            f.write_all(&chunk).await?;
        }

        Ok(file_name)
    }

    async fn parse_calendar(&self, file_name: &str) -> Result<Vec<Task>, Box<dyn Error>> {
        crate::ical::parse_calendar(&self.cache_folder.join(file_name)).await
    }

    fn create_or_update_request_body(&self, storage_type: &StorageType, t: &Task) -> String {
        let mut task = t.clone();

        let task_type = if *storage_type == StorageType::Calendar {
            task.start = task.start.or(task.due);
            task.end = task
                .start
                .map(|d| d.checked_add_signed(DEFAULT_EVENT_DURATION).unwrap_or_default());

            "VEVENT"
        } else {
            "VTODO"
        };

        let properties: Vec<Property> = (&task).into();
        format!(
            r#"BEGIN:VCALENDAR
BEGIN:{task_type}
{}
END:{task_type}
END:VCALENDAR"#,
            properties.iter().map(property_to_str).join("\n")
        )
    }

    fn task_href(&self, t: &Task) -> Result<String, Box<dyn Error>> {
        Ok(if t.href.is_empty() {
            let url = url::Url::parse(&self.cfg.url)?;
            url.join(format!("{}.ics", uuid::Uuid::new_v4()).as_str())?
                .path()
                .to_string()
        } else {
            t.href.clone()
        })
    }

    async fn send_create_or_update_request(
        &mut self,
        storage_type: &StorageType,
        t: &Task,
    ) -> Result<reqwest::Response, Box<dyn Error>> {
        let body = self.create_or_update_request_body(storage_type, t);
        let href = self.task_href(t)?;

        tracing::debug!(body=?body, task=?&t, href=href, "Create or update a task");

        let c = self.client()?;
        c.start_request(Method::from_bytes(b"PUT").unwrap(), &href)
            .await?
            .headers({
                let mut map = HeaderMap::new();
                map.insert("Content-Type", HeaderValue::from_str("text/calendar; charset=utf-8")?);
                map
            })
            .body(body)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

fn file_name_from_href(href: &str) -> Result<String, StringError> {
    if href.is_empty() {
        return Err(StringError::new("empty string"));
    }

    href.rfind('/')
        .map(|pos| href[pos + 1..].to_string())
        .ok_or(StringError::new("wrong href format"))
}

#[cfg(test)]
mod test {
    use super::file_name_from_href;

    #[test]
    fn file_name_from_href_test() {
        struct Case<'a> {
            name: &'a str,
            href: &'a str,
            file_name: Option<&'a str>,
            error: Option<&'a str>,
        }
        const CASES: &[Case] = &[
            Case {
                name: "regular href",
                href: "/remote.php/dav/calendars/user@domain.org/tasks/81CAB8BE-2B83-4ABF-921B-EC697FFC293D.ics",
                file_name: Some("81CAB8BE-2B83-4ABF-921B-EC697FFC293D.ics"),
                error: None,
            },
            Case {
                name: "empty href",
                href: "",
                file_name: None,
                error: Some("empty string"),
            },
        ];

        for c in CASES {
            let r = file_name_from_href(c.href);

            if let Some(name) = c.file_name {
                assert!(r.is_ok());
                assert_eq!(r.as_ref().unwrap(), name, "Test '{}' was failed", c.name);
            }

            if let Some(error) = c.error {
                assert!(r.is_err());
                assert_eq!(
                    r.as_ref().unwrap_err().to_string(),
                    error,
                    "Test '{}' was failed",
                    c.name
                );
            }
        }
    }
}
