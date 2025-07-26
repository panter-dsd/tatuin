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

fn read_config(file_name: path::PathBuf) -> Option<Config> {
    tracing::info!(target:"HERE", path=?file_name);
    if let Ok(data) = fs::read_to_string(file_name) {
        let x: Result<Config, serde_json::Error> = serde_json::from_str(data.as_str());
        tracing::info!(target:"HERE", x=?x);
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

    fn token(&self) -> Result<String, StringError> {
        let cfg = self.cfg.as_ref().ok_or(not_connected_err())?;
        Ok(cfg.api_key.clone())
    }

    fn url(&self, uri: &str) -> Result<String, StringError> {
        let cfg = self.cfg.as_ref().ok_or(not_connected_err())?;
        Ok(format!("https://localhost:{}{uri}", cfg.port))
    }

    pub async fn add_text_to_daily_note(&self, data: &str) -> Result<(), StringError> {
        self.client
            .post(self.url("/periodic/daily")?)
            .bearer_auth(self.token()?)
            .body(reqwest::Body::wrap(data.to_string()))
            .header(reqwest::header::CONTENT_TYPE, "text/markdown")
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!(target:"obsidian_rest_client", data=?data, cfg=?self.cfg, "Add text to daily note");
                StringError::new(e.to_string().as_str())
            })
    }
}
