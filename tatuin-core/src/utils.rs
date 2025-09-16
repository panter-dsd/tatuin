// SPDX-License-Identifier: MIT

use std::process::Command;

use super::StringError;

impl From<std::io::Error> for StringError {
    fn from(e: std::io::Error) -> Self {
        StringError::new(e.to_string().as_str())
    }
}

pub fn open_url(url: &str) -> Result<(), StringError> {
    if cfg!(target_os = "macos") {
        if let Err(e) = Command::new("open").arg(url).status() {
            return Err(e.into());
        }
    } else if cfg!(target_os = "linux") {
        if let Err(e) = Command::new("xdg-open").arg(url).status() {
            return Err(e.into());
        }
    } else {
        return Err(StringError::new("can't open url in target os"));
    };

    Ok(())
}
