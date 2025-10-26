use std::path::Path;

pub fn strip_root_str(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_str().unwrap_or_default())
        .unwrap_or_default()
        .to_string()
}
