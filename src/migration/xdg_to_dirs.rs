use std::io::ErrorKind;

pub fn migrate_config(app_name: &str, file_name: &str) {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
    if let Some(config_file) = xdg_dirs.get_config_file(file_name)
        && std::fs::exists(&config_file).is_ok_and(|r| r)
    {
        let config_dir = dirs::config_dir().expect("Can't detect config dir").join(app_name);
        if let Err(e) = std::fs::create_dir(&config_dir)
            && e.kind() != ErrorKind::AlreadyExists
        {
            panic!("Create a config dir {config_dir:?}: {e}");
        }

        let new_config_file = config_dir.join(file_name);
        if let Err(e) = std::fs::copy(&config_file, &new_config_file) {
            panic!("Copy config from {config_file:?} to {new_config_file:?}: {e}");
        }
        if let Err(e) = std::fs::remove_file(&config_file) {
            panic!("Remove the old config file {config_file:?}: {e}");
        }
    }
}
