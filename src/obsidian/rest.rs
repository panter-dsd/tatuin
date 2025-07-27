// SPDX-License-Identifier: MIT

use serde::Deserialize;
use std::fs;
use std::path;

use crate::provider::StringError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Config {
    port: u16,
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
    pub fn new(vault_path: &str) -> Self {
        Self {
            cfg: read_config(path::Path::new(vault_path).join(CONFIG_PATH)),
            client: reqwest::Client::new(),
        }
    }

    pub fn is_available(&self) -> bool {
        self.cfg.is_some()
    }

    fn token(&self) -> Result<String, StringError> {
        let cfg = self.cfg.as_ref().ok_or(not_connected_err())?;
        Ok(cfg.api_key.clone())
    }

    fn url(&self, uri: &str) -> Result<String, StringError> {
        let cfg = self.cfg.as_ref().ok_or(not_connected_err())?;
        Ok(format!("https://localhost:{}{uri}", cfg.port))
    }

    #[tracing::instrument(level = "info", target = "obsidian_rest_client")]
    pub async fn add_text_to_daily_note(&self, data: &str) -> Result<(), StringError> {
        // Sometimes, if the daily note doesn't exist, it is created but without any sent data.
        // I think, it might be because of temlates applying.
        // So, we send the empty request first to create the note.
        let url = self.url("/periodic/daily")?;
        let token = self.token()?;

        let _ = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "text/markdown")
            .send()
            .await
            .map_err(|e| {
                tracing::error!(target:"obsidian_rest_client", data=?data, cfg=?self.cfg, "Create the daily note");
                StringError::new(e.to_string().as_str())
            })?;
        self.client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "text/markdown")
            .body(reqwest::Body::wrap(data.to_string()))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"obsidian_rest_client", data=?data, cfg=?self.cfg, "Add text to daily note");
                StringError::new(e.to_string().as_str())
            })
    }
}
