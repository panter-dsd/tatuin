// SPDX-License-Identifier: MIT

use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

#[derive(Clone)]
pub enum AcceptResult {
    NotAccepted,
    PartiallyAccepted,
    Accepted,
}

pub struct SharedData {
    pub name: String,
    pub keys: Vec<char>,
    pub is_global: bool,
    current_input_keys: Vec<char>,
}

pub struct Shortcut {
    data: Arc<RwLock<SharedData>>,
    tx: broadcast::Sender<()>,
}

impl Shortcut {
    pub fn new(name: &str, keys: &[char]) -> Self {
        let (tx, _) = broadcast::channel(1);

        Self {
            data: Arc::new(RwLock::new(SharedData {
                name: name.to_string(),
                keys: keys.to_vec(),
                is_global: false,
                current_input_keys: Vec::new(),
            })),
            tx,
        }
    }

    pub fn global(self) -> Self {
        self.data.write().unwrap().is_global = true;
        self
    }

    pub fn is_global(&self) -> bool {
        self.data.read().unwrap().is_global
    }

    pub fn internal_data(&self) -> Arc<RwLock<SharedData>> {
        self.data.clone()
    }

    pub fn subscribe_to_accepted(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    pub fn current_input_keys(&self) -> Vec<char> {
        self.data.read().unwrap().current_input_keys.to_vec()
    }

    pub fn keys(&self) -> Vec<char> {
        self.data.read().unwrap().keys.to_vec()
    }

    pub fn accept(&mut self, keys: &[char]) -> AcceptResult {
        let mut d = self.data.write().unwrap();
        d.current_input_keys.clear();

        if d.keys == keys {
            // We don't care here about the send result.
            // Probably, there is no subscriber here.
            let _ = self.tx.send(());
            AcceptResult::Accepted
        } else if d.keys.starts_with(keys) {
            d.current_input_keys = keys.to_vec();
            AcceptResult::PartiallyAccepted
        } else {
            AcceptResult::NotAccepted
        }
    }
}
