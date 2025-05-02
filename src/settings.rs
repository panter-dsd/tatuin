use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(skip_serializing, skip_deserializing)]
    file_name: String,

    pub providers: HashMap<String, HashMap<String, String>>,
}

impl Settings {
    pub fn new(file_name: &str) -> Self {
        println!("Load config from {file_name}");
        let settings = Config::builder()
            .add_source(File::new(file_name, FileFormat::Toml))
            .build();

        if let Ok(s) = settings {
            return Self {
                file_name: file_name.to_string(),
                ..s.try_deserialize::<Self>().unwrap()
            };
        }

        Self {
            file_name: file_name.to_string(),
            ..Settings::default()
        }
    }

    pub fn add_provider(
        &mut self,
        name: &str,
        config: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.providers.insert(name.to_string(), config.clone());

        let s = toml::to_string(self)?;

        std::fs::write(&self.file_name, s)?;

        Ok(())
    }
}
