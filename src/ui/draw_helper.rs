// SPDX-License-Identifier: MIT

use crate::types::ArcRwLock;
use ratatui::layout::{Constraint, Flex, Layout, Position, Rect, Size};

pub trait DrawHelperTrait: Send + Sync {
    fn redraw(&mut self);
    fn set_cursor_pos(&mut self, pos: Position);
    fn hide_cursor(&mut self);
    fn set_screen_size(&mut self, s: Size);
    fn screen_size(&self) -> Size;
}

pub type DrawHelper = ArcRwLock<Box<dyn DrawHelperTrait>>;

pub fn global_dialog_area(size: Size, area: Rect) -> Rect {
    let [area] = Layout::vertical([Constraint::Length(size.height)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Length(size.width)])
        .flex(Flex::Center)
        .areas(area);
    area
}
