// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crossterm::event::KeyEvent;

#[async_trait]
pub trait KeyboardHandler {
    async fn handle_key(&mut self, key: KeyEvent) -> bool;
}
