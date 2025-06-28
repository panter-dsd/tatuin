// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Modifier, Style},
    text::Text as RatatuiText,
    widgets::Widget,
};

use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler};

use super::WidgetTrait;

pub struct Text {
    text: String,
    width: u16,
    pos: Position,
    style: Style,
    modifier: Modifier,
}

impl Text {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            width: RatatuiText::from(text).width() as u16,
            pos: Position::default(),
            style: Style::default(),
            modifier: Modifier::empty(),
        }
    }

    pub fn modifier(mut self, m: Modifier) -> Self {
        self.modifier = m;
        self
    }
}

#[async_trait]
impl WidgetTrait for Text {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        RatatuiText::styled(self.text.as_str(), self.style.add_modifier(self.modifier)).render(
            Rect {
                x: self.pos.x,
                y: self.pos.y,
                width: area.width,
                height: area.height,
            },
            buf,
        );
    }

    fn size(&self) -> Size {
        Size::new(self.width, 1)
    }

    fn set_pos(&mut self, pos: Position) {
        self.pos = pos
    }

    fn set_style(&mut self, style: Style) {
        self.style = style
    }
}

#[async_trait]
impl KeyboardHandler for Text {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

#[async_trait]
impl MouseHandler for Text {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
