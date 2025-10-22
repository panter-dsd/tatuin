// SPDX-License-Identifier: MIT

use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use tatuin_core::state::{State, StateSettings};

const DEFAULT_STATE_NAME: &str = "default";

#[derive(Serialize, Deserialize)]
pub struct TaskInfoPanel {
    pub description_line_count: usize,
}

impl Default for TaskInfoPanel {
    fn default() -> Self {
        Self {
            description_line_count: 3,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Interface {
    pub task_info_panel: TaskInfoPanel,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(skip_serializing, skip_deserializing)]
    file_name: String,

    pub providers: HashMap<String, HashMap<String, String>>,

    #[serde(default)]
    states: HashMap<String, State>,

    pub theme: Option<String>,

    #[serde(default)]
    pub interface: Interface,
}

impl Settings {
    pub fn new(file_name: &str) -> Self {
        let settings = Config::builder()
            .add_source(File::new(file_name, FileFormat::Toml))
            .build();

        if let Ok(s) = settings {
            return Self {
                file_name: file_name.to_string(),
                ..s.try_deserialize::<Self>().unwrap()
            };
        }

        Self {
            file_name: file_name.to_string(),
            ..Settings::default()
        }
    }

    pub fn add_provider(&mut self, name: &str, config: &HashMap<String, String>) -> Result<(), Box<dyn Error>> {
        self.providers.insert(name.to_string(), config.clone());

        self.save_to_file()
    }

    fn save_to_file(&self) -> Result<(), Box<dyn Error>> {
        let s = toml::to_string(self)?;

        std::fs::write(&self.file_name, s)?;

        Ok(())
    }
}

fn state_name(name: Option<&str>) -> String {
    name.unwrap_or(DEFAULT_STATE_NAME).to_string()
}

impl StateSettings for Settings {
    fn load(&self, name: Option<&str>) -> State {
        self.states.get(&state_name(name)).cloned().unwrap_or(State::new())
    }

    fn save(&mut self, name: Option<&str>, state: State) -> Result<(), Box<dyn Error>> {
        self.states.insert(state_name(name), state);
        self.save_to_file()
    }

    fn remove(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        self.states.remove(name);
        self.save_to_file()
    }

    fn states(&self) -> Vec<String> {
        let mut result: Vec<String> = self.states.keys().cloned().collect();
        result.sort_by(|l, r| {
            if l == DEFAULT_STATE_NAME {
                Ordering::Less
            } else if r == DEFAULT_STATE_NAME {
                Ordering::Greater
            } else {
                l.cmp(r)
            }
        });
        result
    }
}
