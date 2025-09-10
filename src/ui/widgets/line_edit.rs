// SPDX-License-Identifier: MIT

use std::any::Any;

use super::{WidgetState, WidgetStateTrait, WidgetTrait};
use crate::ui::{draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    text::Text,
    widgets::{Block, Paragraph, Widget},
};
use regex::Regex;

pub struct LineEdit {
    text: String,
    validator: Option<Regex>,
    cursor_pos: u16,
    last_cursor_pos: Position,
    draw_helper: Option<DrawHelper>,
    widget_state: WidgetState,
}

impl WidgetStateTrait for LineEdit {
    fn is_active(&self) -> bool {
        self.widget_state.is_active()
    }

    fn set_active(&mut self, is_active: bool) {
        self.widget_state.set_active(is_active);
        if is_active {
            self.last_cursor_pos = Position::default();
        }
    }

    fn is_enabled(&self) -> bool {
        self.widget_state.is_enabled()
    }

    fn set_enabled(&mut self, is_enabled: bool) {
        self.widget_state.set_enabled(is_enabled);
    }

    fn is_visible(&self) -> bool {
        self.widget_state.is_visible()
    }

    fn set_visible(&mut self, is_visible: bool) {
        self.widget_state.set_visible(is_visible);
    }
}

impl LineEdit {
    pub fn new(validator: Option<Regex>) -> Self {
        Self {
            text: String::new(),
            validator,
            draw_helper: None,
            cursor_pos: 0,
            last_cursor_pos: Position::default(),
            widget_state: WidgetState::default(),
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor_pos = text.chars().count() as u16;
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }
}

#[async_trait]
impl WidgetTrait for LineEdit {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::bordered().border_style(style::border_color());

        let inner_area = b.inner(area);
        let mut text = self.text.clone();
        let text_width = Text::from(text.as_str()).width() as u16;

        if text_width >= inner_area.width {
            let count_to_drop = (text_width + 1 - inner_area.width) as usize;
            text.drain(..count_to_drop);
        }

        let mut cursor_pos = self.cursor_pos;

        if cursor_pos >= inner_area.width {
            cursor_pos = inner_area.width - 1;
        }

        Paragraph::new(text.as_str()).block(b).render(area, buf);

        if let Some(dh) = &self.draw_helper
            && self.is_active()
        {
            let pos = Position::new(inner_area.x + cursor_pos, inner_area.y);

            if pos != self.last_cursor_pos {
                dh.write().await.set_cursor_pos(pos);
                self.last_cursor_pos = pos;
            }
        }
    }

    fn size(&self) -> Size {
        Size::new(30, 3)
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.draw_helper = Some(dh)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for LineEdit {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(ch) => {
                let validated = self
                    .validator
                    .as_ref()
                    .is_none_or(|v| v.is_match(format!("{}{ch}", self.text).as_str()));
                if validated {
                    self.text.push(ch);
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Backspace => {
                if !self.text.is_empty() {
                    self.text.pop();
                    self.cursor_pos -= 1;
                }
            }
            _ => {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl MouseHandler for LineEdit {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
