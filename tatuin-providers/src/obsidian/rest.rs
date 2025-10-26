// SPDX-License-Identifier: MIT

use reqwest::StatusCode;
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
        let (transport, port) = if cfg.enable_insecure_server {
            ("http", cfg.insecure_port)
        } else {
            ("https", cfg.port)
        };
        Ok(format!("{transport}://localhost:{port}{uri}"))
    }

    #[tracing::instrument(level = "info", target = "obsidian_rest_client")]
    pub async fn add_text_to_daily_note(&self, data: &str) -> Result<(), StringError> {
        let url = self.url("/periodic/daily")?;
        let token = self.token()?;

        if let Ok(r) = self.client.get(&url).bearer_auth(&token).send().await
            && r.status() == StatusCode::NOT_FOUND
        {
            // Sometimes, when the user have any templating plugin that rewrites all created files with
            // template, the daily note creates with no task. So, we create the daily note first
            // and then add a small delay.
            tracing::info!("Create daily note");

            self.client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "text/markdown")
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"obsidian_rest_client", data=?data, cfg=?self.cfg, error=?e, "Create daily note");
                StringError::new(e.to_string().as_str())
            })?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

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
