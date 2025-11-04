// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use async_trait::async_trait;

pub type State = HashMap<String, String>;

pub fn state_to_str(s: &State) -> Result<String, serde_json::Error> {
    serde_json::to_string(s)
}

pub fn state_from_str(s: &str) -> Result<State, serde_json::Error> {
    serde_json::from_str(s)
}

#[async_trait]
pub trait StatefulObject {
    async fn save(&self) -> State;
    async fn restore(&mut self, state: State);
}

pub trait StateSettings: Send + Sync {
    fn load(&self, name: Option<&str>) -> State;
    fn save(&mut self, name: Option<&str>, state: State) -> Result<(), Box<dyn std::error::Error>>;
    fn remove(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn states(&self) -> Vec<String>;
}
