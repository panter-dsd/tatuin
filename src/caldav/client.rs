use reqwest_dav::{Auth, Client as WebDavClient, ClientBuilder, Depth, list_cmd::ListEntity};
use serde::Serialize;
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

use crate::{provider::StringError, task::DateTimeUtc};

pub struct Config {
    pub url: String,
    pub login: String,
    pub password: String,
}

#[derive(Serialize, Clone)]
struct CachedFile {
    href: String,
    last_modified: DateTimeUtc,
    file_name: String,
}

#[derive(Default, Serialize, Clone)]
struct CachedFiles {
    files: Vec<CachedFile>,
}

pub struct Client {
    cfg: Config,
    c: Option<WebDavClient>,
    cache_folder: PathBuf,
}

impl Client {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            c: None,
            cache_folder: crate::folders::temp_folder(),
        }
    }

    pub fn set_cache_folder(&mut self, p: &Path) {
        self.cache_folder = p.to_path_buf()
    }

    pub async fn download(&mut self) -> Result<(), Box<dyn Error>> {
        let url = self.cfg.url.clone();
        let mut current_cached_files = self.cached_files();
        let mut new_cached_files = CachedFiles::default();

        let c = self.client()?;
        let files = c.list(url.as_str(), Depth::Number(0)).await?;

        for f in files {
            if let ListEntity::File(f) = f {
                if let Some(pos) = current_cached_files.files.iter().position(|cf| cf.href == f.href) {
                    let cached_file = current_cached_files.files.remove(pos);
                    if cached_file.last_modified == f.last_modified {
                        new_cached_files.files.push(cached_file);
                        continue;
                    }
                }

                new_cached_files.files.push(CachedFile {
                    file_name: self.download_and_save_file(f.href.as_str()).await?,
                    href: f.href.to_string(),
                    last_modified: f.last_modified,
                });
            }
        }

        self.clean_missed_files(&current_cached_files.files).await;

        Ok(())
    }
}

impl Client {
    fn client<'a>(&'a mut self) -> Result<&'a WebDavClient, Box<dyn Error>> {
        if self.c.is_none() {
            self.c = Some(
                ClientBuilder::new()
                    .set_host(self.cfg.url.clone())
                    .set_auth(Auth::Basic(self.cfg.login.clone(), self.cfg.password.clone()))
                    .build()?,
            );
        }

        Ok(self.c.as_ref().unwrap())
    }

    fn cached_files(&self) -> CachedFiles {
        if let Ok(s) = std::fs::read_to_string(self.cache_folder.join("index.toml")) {
            toml::from_str(s.as_str()).unwrap_or_default()
        }

        CachedFiles::default()
    }

    async fn clean_missed_files(&self, files: &[CachedFile]) {
        for f in files {
            if let Err(e) = tokio::fs::remove_file(self.cache_folder.join(f.file_name.as_str())).await {
                tracing::error!(error=?e, path=?self.cache_folder, file=f.file_name, "Remove cached file");
            }
        }
    }

    async fn download_and_save_file(&mut self, href: &str) -> Result<String, Box<dyn Error>> {
        let c = self.client()?;
        let mut r = c.get(href).await?;

        let file_name = file_name_from_href(href)?;
        let mut f = tokio::fs::File::create(self.cache_folder.join(file_name.as_str())).await?;
        while let Some(chunk) = r.chunk().await? {
            f.write_all(&chunk).await?;
        }

        Ok(file_name)
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
