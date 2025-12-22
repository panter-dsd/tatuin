// SPDX-License-Identifier: MIT

use crate::{EmojiTransformer, RawLinkTransformer};

pub trait Transformer: std::fmt::Debug {
    fn transform(&self, s: &str) -> String;
}

#[derive(Debug)]
pub struct RichString {
    s: String,
    transformers: Vec<Box<dyn Transformer>>,
}

impl RichString {
    pub fn new(s: &str) -> Self {
        Self {
            s: s.to_string(),
            transformers: vec![Box::new(RawLinkTransformer {}), Box::new(EmojiTransformer {})],
        }
    }

    pub fn with_transformer(mut self, t: Box<dyn Transformer>) -> Self {
        self.transformers.push(t);
        self
    }

    pub fn raw(&self) -> String {
        self.s.clone()
    }

    pub fn display(&self) -> String {
        let mut s = self.raw();
        for t in &self.transformers {
            s = t.transform(&s);
        }
        s
    }
}
