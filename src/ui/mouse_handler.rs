// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crossterm::event::MouseEvent;

#[async_trait]
pub trait MouseHandler: Send + Sync {
    async fn handle_mouse(&mut self, ev: &MouseEvent);
}
