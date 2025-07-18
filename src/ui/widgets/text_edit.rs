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
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

pub struct TextEdit {
    text: String,
    last_cursor_pos: Position,
    draw_helper: Option<DrawHelper>,
    widget_state: WidgetState,
}

impl WidgetStateTrait for TextEdit {
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

impl TextEdit {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            draw_helper: None,
            last_cursor_pos: Position::default(),
            widget_state: WidgetState::default(),
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string()
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }
}

#[async_trait]
impl WidgetTrait for TextEdit {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default().borders(Borders::ALL).border_style(style::BORDER_COLOR);

        let inner_area = b.inner(area);

        let mut lines = self.text.split('\n').collect::<Vec<&str>>();
        let lines_count = lines.len();
        let possible_line_count = inner_area.height - 2;

        let not_all_fit = lines_count > possible_line_count as usize;
        if not_all_fit {
            lines = lines
                .iter()
                .skip(lines.len() - possible_line_count as usize)
                .cloned()
                .collect::<Vec<&str>>();
        }

        Paragraph::new(lines.join("\n")).block(b).render(inner_area, buf);

        if not_all_fit {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(lines_count).position(lines_count);
            scrollbar.render(
                Rect {
                    x: inner_area.x,
                    y: inner_area.y + 1, // header
                    width: inner_area.width,
                    height: inner_area.height - 1,
                },
                buf,
                &mut scrollbar_state,
            );
        }

        if let Some(dh) = &self.draw_helper {
            if self.is_active() {
                let last_line_width = Text::raw(lines[lines.len() - 1]).width() as u16;
                let pos = Position::new(
                    std::cmp::min(inner_area.x + last_line_width + 1, inner_area.x + inner_area.width - 2),
                    std::cmp::min(inner_area.y + lines.len() as u16, inner_area.y + inner_area.height - 2),
                );

                if pos != self.last_cursor_pos {
                    dh.write().await.set_cursor_pos(pos);
                    self.last_cursor_pos = pos;
                }
            }
        }
    }

    fn size(&self) -> Size {
        Size::new(30, 10)
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.draw_helper = Some(dh)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for TextEdit {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(ch) => self.text.push(ch),
            KeyCode::Enter => self.text.push('\n'),
            KeyCode::Backspace => {
                self.text.pop();
            }
            _ => {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl MouseHandler for TextEdit {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
