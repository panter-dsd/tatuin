use std::path::Path;

use tatuin_core::folders;

pub fn migrate_config(app_name: &str, file_name: &str) {
    let config_dir = folders::config_folder(app_name);
    let new_config_file = config_dir.join(file_name);
    if std::fs::exists(&new_config_file).is_ok_and(|r| r) {
        // do nothing because of new configuration exists
        return;
    }

    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
    if let Some(config_file) = xdg_dirs.get_config_file(file_name)
        && std::fs::exists(&config_file).is_ok_and(|r| r)
    {
        println!("Copy the config file {config_file:?} to the new destination {new_config_file:?}");
        if let Err(e) = std::fs::copy(&config_file, &new_config_file) {
            panic!("Copy config from {config_file:?} to {new_config_file:?}: {e}");
        }

        println!("Remove the old config file {config_file:?}");
        if let Err(e) = std::fs::remove_file(&config_file) {
            panic!("Remove the old config file {config_file:?}: {e}");
        }

        migrate_themes(app_name);
        migrate_cache(app_name);
    }
}

fn migrate_themes(app_name: &str) {
    println!("Migrate theme files to the new location");

    let config_folder = xdg::BaseDirectories::with_prefix(app_name)
        .get_config_home()
        .expect("Can't get config folder");
    if let Ok(read_dir) = std::fs::read_dir(&config_folder) {
        for entry in read_dir {
            if let Ok(e) = entry
                && let p = e.path()
                && p.is_file()
                && p.extension().unwrap_or_default() == "theme"
            {
                let new_name = folders::config_folder(app_name).join(p.file_name().expect("Can't get a file name"));

                println!("Copy the theme file {p:?} to the new destination {new_name:?}");
                if let Err(e) = std::fs::copy(&p, &new_name) {
                    panic!("Copy theme file from {p:?} to {new_name:?}: {e}");
                }

                println!("Remove the old theme file {p:?}");
                if let Err(e) = std::fs::remove_file(&p) {
                    panic!("Remove the theme file {p:?} from old location: {e}");
                }
            }
        }
    }
}

fn migrate_cache(app_name: &str) {
    println!("Migrate provider's cache");

    let old_cache = xdg::BaseDirectories::with_prefix(app_name)
        .get_cache_home()
        .expect("Can't get old cache path");
    if !std::fs::exists(&old_cache).is_ok_and(|r| r) {
        return;
    }

    let new_cache = folders::cache_folder(app_name);

    println!("Copy the folder {old_cache:?} to the new destination {new_cache:?}");
    if let Err(e) = copy_recursively(&old_cache, &new_cache) {
        panic!("Copy cache folder {old_cache:?} to {new_cache:?}: {e}");
    }

    println!("Remove the old cache {old_cache:?}");
    if let Err(e) = std::fs::remove_dir_all(&old_cache) {
        panic!("Remove old cache folder {old_cache:?}: {e}");
    }
}

fn copy_recursively(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
