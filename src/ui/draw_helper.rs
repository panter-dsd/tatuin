use std::sync::Arc;

use async_trait::async_trait;
use ratatui::layout::Position;
use tokio::sync::RwLock;

#[async_trait]
pub trait DrawHelperTrait: Send + Sync {
    fn redraw(&mut self);
    async fn set_cursor_pos(&mut self, pos: Position);
}

pub type DrawHelper = Arc<RwLock<Box<dyn DrawHelperTrait>>>;
