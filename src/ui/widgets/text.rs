use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    text::Text as RatatuiText,
    widgets::Widget,
};

use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler};

use super::WidgetTrait;

pub struct Text {
    text: String,
    width: u16,
}

impl Text {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            width: RatatuiText::from(text).width() as u16,
        }
    }
}

#[async_trait]
impl WidgetTrait for Text {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        RatatuiText::from(self.text.as_str()).render(area, buf);
    }

    fn size(&self) -> Size {
        Size::new(self.width, 1)
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
