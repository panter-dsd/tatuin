// SPDX-License-Identifier: MIT

use crate::provider::ProviderTrait;
use std::{io::ErrorKind, path::PathBuf};

pub fn cache_folder(app_name: &str) -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
    xdg_dirs.create_cache_directory("").expect("cannot create cache folder")
}

pub fn state_folder(app_name: &str) -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
    xdg_dirs.create_state_directory("").expect("cannot create state folder")
}

pub fn log_folder(app_name: &str) -> PathBuf {
    state_folder(app_name)
}

pub fn config_folder(app_name: &str) -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
    xdg_dirs.get_config_home().expect("cannot create config folder")
}

pub fn provider_cache_folder(app_name: &str, p: &dyn ProviderTrait) -> Result<PathBuf, std::io::Error> {
    let path = cache_folder(app_name).join(p.name());
    if let Err(e) = std::fs::create_dir(&path)
        && e.kind() != ErrorKind::AlreadyExists
    {
        return Err(e);
    }

    Ok(path)
}

pub fn temp_folder() -> PathBuf {
    std::env::temp_dir()
}
