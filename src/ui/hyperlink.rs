// SPDX-License-Identifier: MIT

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Widget;

pub struct Hyperlink<'content> {
    text: Text<'content>,
    url: String,
}

impl<'content> Hyperlink<'content> {
    pub fn new(text: impl Into<Text<'content>>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }
}

impl Widget for &Hyperlink<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let x = area.x + area.width - (self.text.width() as u16);

        for (i, two_chars) in self.text.to_string().as_bytes().chunks(2).enumerate() {
            let text = String::from_utf8_lossy(two_chars);
            let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
            buffer[(x + i as u16 * 2, area.y)].set_symbol(hyperlink.as_str());
        }
    }
}
