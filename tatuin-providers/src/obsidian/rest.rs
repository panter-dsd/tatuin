// SPDX-License-Identifier: MIT

use reqwest::{StatusCode, header};
use serde::Deserialize;
use std::fs;
use std::path;
use std::path::Path;

use tatuin_core::StringError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Config {
    port: u16,
    insecure_port: u16,
    enable_insecure_server: bool,
    api_key: String,
}

const CONFIG_PATH: &str = ".obsidian/plugins/obsidian-local-rest-api/data.json";

fn not_connected_err() -> StringError {
    StringError::new("the vault doesn't contain the obsidian-local-rest-api plugin")
}

pub struct Client {
    cfg: Option<Config>,
    client: reqwest::Client,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rest client")
    }
}

fn read_config(file_name: path::PathBuf) -> Option<Config> {
    if let Ok(data) = fs::read_to_string(file_name) {
        serde_json::from_str(data.as_str()).ok()
    } else {
        None
    }
}

impl Client {
    pub fn new(vault_path: &Path) -> Self {
        Self {
            cfg: read_config(vault_path.join(CONFIG_PATH)),
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("Failed to build the HTTP client"),
        }
    }

    pub fn is_available(&self) -> bool {
        self.cfg.is_some()
    }

    #[tracing::instrument(level = "info", target = "obsidian_rest_client")]
    pub async fn add_text_to_daily_note(&self, data: &str) -> Result<(), StringError> {
        let url = self.daily_note_url().await.map_err(|e| {
            tracing::error!(error=?e, "Get the daily note url");
            StringError::new(&e.to_string())
        })?;
        let token = self.token()?;

        self.client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "text/markdown")
            .body(reqwest::Body::wrap(data.to_string()))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"obsidian_rest_client", data=?data, cfg=?self.cfg, error=?e, "Add text to daily note");
                StringError::new(e.to_string().as_str())
            })
    }
}

impl Client {
    fn token(&self) -> Result<String, StringError> {
        let cfg = self.cfg.as_ref().ok_or(not_connected_err())?;
        Ok(cfg.api_key.clone())
    }

    fn url(&self, uri: &str) -> Result<String, StringError> {
        let cfg = self.cfg.as_ref().ok_or(not_connected_err())?;
        let (transport, port) = if cfg.enable_insecure_server {
            ("http", cfg.insecure_port)
        } else {
            ("https", cfg.port)
        };
        Ok(format!("{transport}://localhost:{port}{uri}"))
    }

    #[tracing::instrument(level = "info", target = "obsidian_rest_client")]
    async fn daily_note_url(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = self.url("/periodic/daily/")?;
        let token = self.token()?;

        tracing::info!("Create or get daily note");

        let r = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "text/markdown")
            .send()
            .await
            .map_err(|e| {
                tracing::error!(target:"obsidian_rest_client",  cfg=?self.cfg, error=?e, "Create daily note");
                StringError::new(e.to_string().as_str())
            })?;
        match r.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(url),
            StatusCode::TEMPORARY_REDIRECT => {
                let location = r.headers().get(header::LOCATION).ok_or("there is no location header")?;
                self.url(location.to_str()?).map_err(Into::into)
            }
            _ => Err(format!("wrong status code: {:?}", r.status()).into()),
        }
    }
}
