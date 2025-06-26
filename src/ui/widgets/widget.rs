// SPDX-License-Identifier: MIT

use crate::ui::{draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler};

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
};

use async_trait::async_trait;

#[async_trait]
pub trait WidgetTrait: KeyboardHandler + MouseHandler + Send + Sync {
    async fn render(&mut self, area: Rect, buf: &mut Buffer);
    fn size(&self) -> Size;
    fn set_draw_helper(&mut self, _dh: DrawHelper) {}
    fn set_pos(&mut self, _pos: Position) {}
}
