// SPDX-License-Identifier: MIT

use crate::types::ArcRwLock;
use ratatui::layout::Position;

pub trait DrawHelperTrait: Send + Sync {
    fn redraw(&mut self);
    fn set_cursor_pos(&mut self, pos: Position);
    fn hide_cursor(&mut self);
}

pub type DrawHelper = ArcRwLock<Box<dyn DrawHelperTrait>>;
