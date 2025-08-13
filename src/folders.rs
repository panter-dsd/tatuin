use crate::APP_NAME;
use std::path::PathBuf;

pub fn cache_folder() -> PathBuf {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    xdg_dirs
        .create_state_directory("cache")
        .expect("cannot create cache folder")
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

