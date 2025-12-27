// SPDX-License-Identifier: MIT

use std::sync::LazyLock;

use regex::Regex;

use crate::RichStringTransformerTrait;

static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(:([\w+-]+):)").unwrap());

#[derive(Debug)]
pub struct EmojiTransformer {}

impl RichStringTransformerTrait for EmojiTransformer {
    fn transform(&self, s: &str) -> String {
        let mut result = String::new();
        let mut last_end: usize = 0;

        for cap in RE.captures_iter(s) {
            let (full, [_, emoji_shortcode]) = cap.extract();
            let m = cap.get_match();

            result.push_str(&s[last_end..m.start()]);
            if let Some(emoji) = emojis::get_by_shortcode(emoji_shortcode) {
                result.push_str(emoji.as_str());
            } else {
                result.push_str(full);
            }

            last_end = m.end();
        }

        result.push_str(&s[last_end..]);
        result
    }
}

#[cfg(test)]
mod test {
    use crate::{EmojiTransformer, RichStringTransformerTrait};

    #[test]
    fn single_emoji() {
        assert_eq!("😄", EmojiTransformer {}.transform(":smile:"));
    }

    #[test]
    fn inside_string() {
        assert_eq!(
            "Some text 😄 some text",
            EmojiTransformer {}.transform("Some text :smile: some text")
        );
    }

    #[test]
    fn unknown_emoji() {
        assert_eq!(
            "Some text :abrakadabra: some text",
            EmojiTransformer {}.transform("Some text :abrakadabra: some text")
        );
    }
}
