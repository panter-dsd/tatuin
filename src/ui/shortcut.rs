#[derive(Clone)]

pub enum AcceptResult {
    NotAccepted,
    PartiallyAccepted,
    Accepted,
}

#[derive(Clone)]
pub struct Shortcut {
    keys: Vec<char>,
}

impl Shortcut {
    pub fn new(keys: &[char]) -> Self {
        Self {
            keys: keys.to_vec(),
        }
    }

    pub fn text(&self) -> String {
        self.keys.iter().collect()
    }

    pub fn accept(&self, keys: &[char]) -> AcceptResult {
        if self.keys == keys {
            AcceptResult::Accepted
        } else if self.keys.starts_with(keys) {
            AcceptResult::PartiallyAccepted
        } else {
            AcceptResult::NotAccepted
        }
    }
}
