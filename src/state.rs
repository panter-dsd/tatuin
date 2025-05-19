use std::collections::HashMap;

pub type State = HashMap<String, String>;

pub fn state_to_str(s: &State) -> Result<String, Box<dyn std::error::Error>> {
    match toml::to_string(s) {
        Ok(v) => Ok(v),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn state_from_str(s: &str) -> Result<State, Box<dyn std::error::Error>> {
    match toml::from_str(s) {
        Ok(v) => Ok(v),
        Err(e) => Err(Box::new(e)),
    }
}

pub trait StatefulObject {
    fn save(&self) -> State;
    fn restore(&mut self, state: State);
}

pub trait StateSettings {
    fn load(&self, name: Option<&str>) -> State;
    fn save(&mut self, name: Option<&str>, state: State) -> Result<(), Box<dyn std::error::Error>>;
    fn remove(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn rename(&mut self, old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn states(&self) -> Vec<String>;
}
