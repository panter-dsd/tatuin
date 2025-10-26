use std::path::{Path, PathBuf};

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
