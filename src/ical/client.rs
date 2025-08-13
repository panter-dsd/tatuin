use std::path::{Path, PathBuf};

pub struct Client {
    url: String,
    cache_folder: PathBuf,
}

impl Client {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            cache_folder: crate::folders::temp_folder(),
        }
    }

    pub fn set_cache_folder(&mut self, p: &Path) {
        self.cache_folder = p.to_path_buf();
        println!("HERE: {}", self.cache_folder.to_str().unwrap_or_default())
    }
}
