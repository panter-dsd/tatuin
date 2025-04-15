use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Obsidian {
    pub path: String,
}

#[derive(Deserialize)]
pub struct Settings {
    pub obsidian: Obsidian,
}

impl Settings {
    pub fn load(file_name: &str) -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::new(file_name, FileFormat::Toml))
            .build()?;
        let mut s = settings.try_deserialize::<Settings>()?;
        if !s.obsidian.path.ends_with('/') {
            s.obsidian.path.push('/')
        }
        Ok(s)
    }
}
