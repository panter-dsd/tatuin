// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{
    de::Deserialize,
    ser::{Serialize, SerializeMap},
};

#[derive(Clone, Default)]
pub struct State(HashMap<String, String>);

impl State {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.0.insert(key.to_string(), value.to_string());
    }

    pub fn insert_str(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }

    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.0
    }
}

pub fn state_from_str(s: &str) -> Result<State, serde_json::Error> {
    serde_json::from_str(s)
}

impl From<State> for String {
    fn from(s: State) -> Self {
        serde_json::to_string(&s.0).unwrap()
    }
}

impl<const N: usize> From<[(String, String); N]> for State {
    fn from(v: [(String, String); N]) -> Self {
        Self(HashMap::from(v))
    }
}

impl Serialize for State {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            s.serialize_entry(k, v)?;
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for State {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(HashMap::<String, String>::deserialize(deserializer)?))
    }
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
