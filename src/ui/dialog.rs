use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};

use async_trait::async_trait;
use std::any::Any;

#[async_trait]
pub trait DialogTrait {
    async fn render(&mut self, area: Rect, buf: &mut Buffer);
    async fn handle_key(&mut self, key: KeyEvent);
    fn should_be_closed(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn size(&self) -> Size;
}
