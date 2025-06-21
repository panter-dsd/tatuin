// SPDX-License-Identifier: MIT

use std::sync::Arc;

use ratatui::layout::Position;
use tokio::sync::RwLock;

pub trait DrawHelperTrait: Send + Sync {
    fn redraw(&mut self);
    fn set_cursor_pos(&mut self, pos: Position);
    fn hide_cursor(&mut self);
}

pub type DrawHelper = Arc<RwLock<Box<dyn DrawHelperTrait>>>;
