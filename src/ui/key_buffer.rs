// SPDX-License-Identifier: MIT

use std::fmt::Write;

#[derive(Default)]
pub struct KeyBuffer {
    keys: Vec<char>,
}

impl KeyBuffer {
    pub fn push(&mut self, key: char) -> Vec<char> {
        const MAX_KEYS_COUNT: usize = 2;
        if self.keys.len() == MAX_KEYS_COUNT {
            self.clear();
        }
        self.keys.push(key);
        self.keys.to_vec()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl std::fmt::Display for KeyBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in &self.keys {
            f.write_char(*c)?
        }

        Ok(())
    }
}
