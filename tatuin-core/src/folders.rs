// SPDX-License-Identifier: MIT

use std::{io::ErrorKind, path::PathBuf};

pub fn cache_folder(app_name: &str) -> PathBuf {
    let p = dirs::cache_dir().expect("Can't detect cache folder").join(app_name);
    create_dir(&p);
    p
}

pub fn log_folder(app_name: &str) -> PathBuf {
    let p = if cfg!(target_os = "macos") {
        dirs::home_dir()
            .expect("Can't detect home folder")
            .join("Library/Logs")
            .join(app_name)
    } else if cfg!(target_os = "linux") {
        dirs::state_dir().expect("Can't detect log folder").join(app_name)
    } else if cfg!(target_os = "windows") {
        dirs::cache_dir().expect("Can't detect log folder").join(app_name)
    } else {
        dirs::state_dir().expect("Can't detect log folder").join(app_name)
    };
    create_dir(&p);
    p
}

pub fn config_folder(app_name: &str) -> PathBuf {
    let p = dirs::config_dir().expect("Can't detect config dir").join(app_name);
    create_dir(&p);
    p
}

pub fn provider_cache_folder(app_name: &str, provider_name: &str) -> std::io::Result<PathBuf> {
    let path = cache_folder(app_name).join(provider_name);
    try_create_dir(&path)?;
    Ok(path)
}

pub fn temp_folder() -> PathBuf {
    std::env::temp_dir()
}

pub fn create_dir(p: &PathBuf) {
    if let Err(e) = try_create_dir(p) {
        panic!("Can't create the path {p:?}: {e}");
    }
}

pub fn try_create_dir(p: &PathBuf) -> std::io::Result<()> {
    if let Err(e) = std::fs::create_dir_all(p)
        && e.kind() != ErrorKind::AlreadyExists
    {
        return Err(e);
    }

    Ok(())
}
