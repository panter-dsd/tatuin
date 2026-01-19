// SPDX-License-Identifier: MIT

use std::{any::Any, ops::Sub};

use super::{WidgetState, WidgetStateTrait, WidgetTrait};
use crate::ui::{draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    text::Text,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

pub struct TextEdit {
    lines: Vec<String>,
    current_line: usize,
    pos_in_line: usize,
    last_cursor_pos: Position,
    draw_helper: Option<DrawHelper>,
    widget_state: WidgetState,
    size: Size,
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

#[allow(dead_code)]
impl TextEdit {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_line: 0,
            pos_in_line: 0,
            draw_helper: None,
            last_cursor_pos: Position::default(),
            widget_state: WidgetState::default(),
            size: Size::default(),
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text(&mut self, text: &str) {
        self.lines = text.split('\n').map(|s| s.to_string()).collect::<Vec<String>>();
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_line = 0;
        self.pos_in_line = 0;
    }
}

#[async_trait]
impl WidgetTrait for TextEdit {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::bordered().border_style(style::border_color());

        let inner_area = b.inner(area);

        let mut lines = self.lines.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        let lines_count = lines.len();
        let possible_line_count = inner_area.height;

        let not_all_fit = lines_count > possible_line_count as usize;
        if not_all_fit {
            lines.drain(0..{ lines_count - possible_line_count as usize });
        }

        Paragraph::new(lines.join("\n")).block(b).render(area, buf);

        if not_all_fit {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(lines_count).position(lines_count);
            scrollbar.render(
                Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: area.height,
                },
                buf,
                &mut scrollbar_state,
            );
        }

        if let Some(dh) = &self.draw_helper
            && self.is_active()
        {
            let last_line_width = Text::raw(*lines.get(lines.len().saturating_sub(1)).unwrap_or(&"")).width() as u16;
            let pos = Position::new(
                std::cmp::min(inner_area.x + last_line_width, inner_area.x + inner_area.width - 1),
                std::cmp::min(
                    inner_area.y + lines.len().saturating_sub(1) as u16,
                    inner_area.y + inner_area.height,
                ),
            );
            if pos != self.last_cursor_pos {
                dh.write().await.set_cursor_pos(pos, None);
                self.last_cursor_pos = pos;
            }
        }
    }

    fn min_size(&self) -> Size {
        Size::new(5, 1)
    }

    fn size(&self) -> Size {
        self.size
    }

    fn set_size(&mut self, size: Size) {
        let min_size = self.min_size();
        self.size.width = min_size.width.max(size.width);
        self.size.height = min_size.height.max(size.height);
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
            KeyCode::Char(ch) => {
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }

                let s = self.lines.get_mut(self.current_line).unwrap();
                s.push(ch);
                self.pos_in_line += 1;
            }
            KeyCode::Enter => {
                self.lines.push(String::new());
                self.current_line = self.lines.len().sub(1);
                self.pos_in_line = 0;
            }
            KeyCode::Backspace if !self.lines.is_empty() => {
                let s = self.lines.get_mut(self.current_line).unwrap();
                if s.is_empty() {
                    self.lines.pop();
                    self.current_line = self.current_line.saturating_sub(1);
                    self.pos_in_line = self
                        .lines
                        .get(self.current_line)
                        .map(|l| l.len().saturating_sub(1))
                        .unwrap_or(0);
                } else {
                    s.pop();
                    self.pos_in_line = self.pos_in_line.saturating_sub(1);
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
impl MouseHandler for TextEdit {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
