// SPDX-License-Identifier: MIT

use std::any::Any;

use super::DialogTrait;
use crate::ui::{
    draw_helper::DrawHelper,
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    style,
    widgets::LineEdit,
    widgets::{State, StateTrait, WidgetTrait},
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect, Size},
    text::Text,
    widgets::{Block, Borders, Widget},
};
use regex::Regex;

const FOOTER: &str = "Input text and press Enter for applying or Esc for cancelling";

pub struct Dialog {
    title: String,
    edit: LineEdit,
    should_be_closed: bool,
    draw_helper: Option<DrawHelper>,
    state: State,
}
crate::impl_state_trait!(Dialog);

impl Dialog {
    pub fn new(title: &str, input_re: Regex) -> Self {
        Self {
            title: title.to_string(),
            edit: LineEdit::new(Some(input_re)),
            should_be_closed: false,
            draw_helper: None,
            state: State::default(),
        }
    }

    pub fn text(&self) -> String {
        self.edit.text()
    }
}

#[async_trait]
impl WidgetTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_top(self.title.clone())
            .title_bottom(FOOTER)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        let inner_area = b.inner(area);
        b.render(area, buf);
        self.edit.render(inner_area, buf).await;
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.edit.set_draw_helper(dh.clone());
        self.draw_helper = Some(dh);
    }

    fn size(&self) -> Size {
        let edit_size = self.edit.size();
        Size::new(Text::from(FOOTER).width() as u16 + 2, edit_size.height + 2)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.edit.handle_key(key).await {
            return true;
        }

        match key.code {
            KeyCode::Esc => {
                self.should_be_closed = true;
                self.edit.clear();
            }
            KeyCode::Enter => {
                self.should_be_closed = true;
            }
            _ => {
                return false;
            }
        }

        if self.should_be_closed && self.draw_helper.is_some() {
            self.draw_helper.as_ref().unwrap().write().await.hide_cursor();
        }
        true
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
