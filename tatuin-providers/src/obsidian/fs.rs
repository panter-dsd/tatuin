use std::path::{Path, PathBuf};

use urlencoding::encode;

pub fn strip_root_str(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_str().unwrap_or_default())
        .unwrap_or_default()
        .to_string()
}

pub fn supported_files(p: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut result = Vec::new();

    for e in std::fs::read_dir(p)? {
        let entry = e?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            result.push(path);
        } else if path.is_dir() {
            let mut files = supported_files(path.as_path())?;
            result.append(&mut files);
        }
    }

    Ok(result)
}

pub fn obsidian_url(vault: &Path, file: &Path) -> String {
    vault
        .file_name()
        .and_then(|s| s.to_str())
        .map(|vault_name| {
            format!(
                "obsidian://open?vault={}&file={}",
                vault_name,
                encode(strip_root_str(vault, file).as_str())
            )
        })
        .unwrap_or_default()
}

/*
* find file by name or by relative path
*/
pub fn find_file(p: &Path, name: &str) -> Result<PathBuf, std::io::Error> {
    for e in std::fs::read_dir(p)? {
        let entry = e?;
        let path = entry.path();
        if path.is_file() && path.ends_with(name) {
            return Ok(path);
        } else if path.is_dir()
            && let Ok(f) = find_file(path.as_path(), name)
        {
            return Ok(f);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "requested file not found",
    ))
}
