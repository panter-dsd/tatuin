use config::{Config, File, FileFormat};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Default)]
pub struct Settings {
    pub providers: HashMap<String, HashMap<String, String>>,
}

impl Settings {
    pub fn new(file_name: &str) -> Self {
        println!("Load config from {file_name}");
        let settings = Config::builder()
            .add_source(File::new(file_name, FileFormat::Toml))
            .build();

        if let Ok(s) = settings {
            s.try_deserialize::<Self>().unwrap_or_default();
        }

        Settings::default()
    }
}
