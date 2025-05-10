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
}

impl Shortcut {
    pub fn new(keys: &[char]) -> Self {
        Self {
            keys: keys.to_vec(),
            current_input_keys: Vec::new(),
        }
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
            AcceptResult::Accepted
        } else if self.keys.starts_with(keys) {
            self.current_input_keys = keys.to_vec();
            AcceptResult::PartiallyAccepted
        } else {
            AcceptResult::NotAccepted
        }
    }
}
