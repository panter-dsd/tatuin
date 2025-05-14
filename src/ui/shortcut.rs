// SPDX-License-Identifier: MIT

use tokio::sync::broadcast;

#[derive(Clone)]
pub enum AcceptResult {
    NotAccepted,
    PartiallyAccepted,
    Accepted,
}

#[derive(Clone)]
pub struct Shortcut {
    keys: Vec<char>,
    current_input_keys: Vec<char>,
    tx: broadcast::Sender<()>,
}

impl Shortcut {
    pub fn new(keys: &[char]) -> Self {
        let (tx, _) = broadcast::channel(1);

        Self {
            keys: keys.to_vec(),
            current_input_keys: Vec::new(),
            tx,
        }
    }

    pub fn subscribe_to_accepted(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    pub fn current_input_keys(&self) -> Vec<char> {
        self.current_input_keys.to_vec()
    }

    pub fn keys(&self) -> Vec<char> {
        self.keys.to_vec()
    }

    pub fn accept(&mut self, keys: &[char]) -> AcceptResult {
        self.current_input_keys.clear();

        if self.keys == keys {
            // We don't care here about the send result.
            // Probably, there is no subscriber here.
            let _ = self.tx.send(());
            AcceptResult::Accepted
        } else if self.keys.starts_with(keys) {
            self.current_input_keys = keys.to_vec();
            AcceptResult::PartiallyAccepted
        } else {
            AcceptResult::NotAccepted
        }
    }
}
