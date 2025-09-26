use std::path::PathBuf;

use tatuin_core::folders;

pub struct Config {
    app_name: String,
    name: String,

    #[cfg(test)]
    pub cache_path: PathBuf,
}

impl Config {
    pub fn new(app_name: &str, name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            name: name.to_string(),

            #[cfg(test)]
            cache_path: PathBuf::default(),
        }
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn cache_path(&self) -> Result<PathBuf, std::io::Error> {
        #[cfg(test)]
        {
            return Ok(self.cache_path.clone());
        }

        #[allow(unreachable_code)]
        folders::provider_cache_folder(self.app_name.as_str(), self.name.as_str())
    }
}
