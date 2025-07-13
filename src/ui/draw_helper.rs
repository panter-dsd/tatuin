// SPDX-License-Identifier: MIT

use crate::types::ArcRwLock;
use ratatui::layout::{Position, Rect, Size};

pub trait DrawHelperTrait: Send + Sync {
    fn redraw(&mut self);
    fn set_cursor_pos(&mut self, pos: Position);
    fn hide_cursor(&mut self);
}

pub type DrawHelper = ArcRwLock<Box<dyn DrawHelperTrait>>;

pub fn global_dialog_area(size: Size, area: Rect) -> Rect {
    Rect {
        x: area.width / 2 - size.width / 2,
        y: area.height / 2 - size.height / 2,
        width: size.width,
        height: size.height,
    }
}
