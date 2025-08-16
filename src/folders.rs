// SPDX-License-Identifier: MIT

use crate::{APP_NAME, provider::ProviderTrait};
use std::{io::ErrorKind, path::PathBuf};

pub fn cache_folder() -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    xdg_dirs.create_cache_directory("").expect("cannot create cache folder")
}

pub fn state_folder() -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    xdg_dirs.create_state_directory("").expect("cannot create state folder")
}

pub fn log_folder() -> PathBuf {
    state_folder()
}

pub fn config_folder() -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    xdg_dirs.get_config_home().expect("cannot create config folder")
}

pub fn provider_cache_folder(p: &dyn ProviderTrait) -> Result<PathBuf, std::io::Error> {
    let path = cache_folder().join(p.name());
    if let Err(e) = std::fs::create_dir(&path) {
        if e.kind() != ErrorKind::AlreadyExists {
            return Err(e);
        }
    }

    Ok(path)
}

pub fn temp_folder() -> PathBuf {
    std::env::temp_dir()
}
