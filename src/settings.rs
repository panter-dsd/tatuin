use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Settings {
    pub providers: HashMap<String, HashMap<String, String>>,
}

impl Settings {
    pub fn load(file_name: &str) -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::new(file_name, FileFormat::Toml))
            .build()?;

        Ok(settings.try_deserialize::<Settings>().unwrap())
    }
}
