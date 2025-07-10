// SPDX-License-Identifier: MIT

use std::any::Any;

use crate::ui::{draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler};

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::Style,
};

use async_trait::async_trait;

#[async_trait]
pub trait WidgetTrait: KeyboardHandler + MouseHandler + Send + Sync {
    async fn render(&mut self, area: Rect, buf: &mut Buffer);
    fn size(&self) -> Size;
    fn set_draw_helper(&mut self, _dh: DrawHelper) {}
    fn pos(&self) -> Position {
        Position::default()
    }
    fn set_pos(&mut self, _pos: Position) {}
    fn style(&self) -> Style {
        Style::default()
    }
    fn set_style(&mut self, _style: Style) {}
    fn area(&self) -> Rect {
        let pos = self.pos();
        let s = self.size();
        Rect {
            x: pos.x,
            y: pos.y,
            width: s.width,
            height: s.height,
        }
    }
    fn is_active(&self) -> bool {
        false
    }
    fn set_active(&mut self, _is_active: bool) {}
    fn as_any(&self) -> &dyn Any;
}
