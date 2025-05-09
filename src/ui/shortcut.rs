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
}
