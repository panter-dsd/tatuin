// SPDX-License-Identifier: MIT

use std::{any::Any, ops::Sub};

use super::{WidgetState, WidgetStateTrait, WidgetTrait};
use crate::ui::{
    draw_helper::{CursorStyle, DrawHelper},
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    style,
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

pub struct TextEdit {
    lines: Vec<String>,
    current_line: usize,
    pos_in_line: usize,
    top_render_line: usize,
    left_render_symbol: usize,
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

impl TextEdit {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_line: 0,
            pos_in_line: 0,
            draw_helper: None,
            top_render_line: 0,
            left_render_symbol: 0,
            last_cursor_pos: Position::default(),
            widget_state: WidgetState::default(),
            size: Size::default(),
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text(&mut self, text: &str) {
        self.clear();
        self.lines = text.split('\n').map(|s| s.to_string()).collect::<Vec<String>>();
        self.current_line = self.lines.len().sub(1);
        self.pos_in_line = self.end_of_current_line();
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_line = 0;
        self.pos_in_line = 0;
        self.top_render_line = 0;
        self.left_render_symbol = 0;
    }
}

impl TextEdit {
    fn current_line(&self) -> Option<&str> {
        self.lines.get(self.current_line).map(|s| s.as_str())
    }

    fn current_line_size(&self) -> usize {
        self.current_line().map(|l| l.chars().count()).unwrap_or(0)
    }

    fn end_of_current_line(&self) -> usize {
        self.current_line_size()
    }

    fn calculate_top_render_line(&mut self, line_count: usize) -> usize {
        if self.top_render_line > self.current_line {
            self.top_render_line = self.current_line;
        } else if self.current_line - self.top_render_line >= line_count {
            self.top_render_line = self.current_line - line_count + 1;
        }

        self.top_render_line
    }

    fn calculate_left_render_symbol(&mut self, max_count: usize) -> usize {
        if self.left_render_symbol > self.pos_in_line {
            self.left_render_symbol = self.pos_in_line;
        } else if self.pos_in_line - self.left_render_symbol >= max_count {
            self.left_render_symbol = self.pos_in_line - max_count + 1;
        } else if self.current_line_size() - self.left_render_symbol < max_count {
            self.left_render_symbol = self.pos_in_line.checked_sub(max_count).map(|v| v + 1).unwrap_or(0);
        }

        self.left_render_symbol
    }
}

#[async_trait]
impl WidgetTrait for TextEdit {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::bordered().border_style(style::border_color());

        let inner_area = b.inner(area);

        let lines_count = self.lines.len();
        let possible_line_count = inner_area.height as usize;

        let top_render_line = self.calculate_top_render_line(possible_line_count);
        let max_symbol_count = inner_area.width as usize;
        let left_render_symbol = self.calculate_left_render_symbol(max_symbol_count);

        let visible_lines = self
            .lines
            .iter()
            .skip(top_render_line)
            .take(possible_line_count)
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();

        let lines = visible_lines
            .iter()
            .map(|s| {
                s.chars()
                    .skip(left_render_symbol)
                    .take(max_symbol_count)
                    .collect::<String>()
            })
            .collect::<Vec<String>>();

        Paragraph::new(lines.join("\n")).block(b).render(area, buf);

        if lines_count != lines.len() {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(lines_count).position(self.current_line);
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

        let longest_line_size = visible_lines
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or_default();

        if longest_line_size >= max_symbol_count {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .begin_symbol(Some("←"))
                .end_symbol(Some("→"));
            let mut scrollbar_state = ScrollbarState::new(longest_line_size).position(self.pos_in_line);
            scrollbar.render(
                Rect {
                    x: area.x + 1,
                    y: area.y,
                    width: area.width - 2,
                    height: area.height,
                },
                buf,
                &mut scrollbar_state,
            );
        }

        if let Some(dh) = &self.draw_helper
            && self.is_active()
        {
            let pos = Position::new(
                std::cmp::min(
                    inner_area.x + (self.pos_in_line - self.left_render_symbol) as u16,
                    inner_area.x + inner_area.width - 1,
                ),
                std::cmp::min(
                    inner_area.y + self.current_line.saturating_sub(self.top_render_line) as u16,
                    inner_area.y + inner_area.height,
                ),
            );
            if pos != self.last_cursor_pos {
                let cursor_style = if self.pos_in_line == self.end_of_current_line() {
                    CursorStyle::BlinkingBlock
                } else {
                    CursorStyle::BlinkingBar
                };
                dh.write().await.set_cursor_pos(pos, Some(cursor_style));
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
        let is_at_end_of_current_line = self.pos_in_line == self.end_of_current_line();

        match key.code {
            KeyCode::Char(ch) => {
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }

                let end_of_current_line = self.end_of_current_line();
                let s = self.lines.get_mut(self.current_line).unwrap();
                if self.pos_in_line == end_of_current_line {
                    s.push(ch);
                } else {
                    s.insert(s.char_indices().nth(self.pos_in_line).unwrap().0, ch);
                }
                self.pos_in_line += 1;
            }
            KeyCode::Enter => match self.lines.get_mut(self.current_line) {
                None => {
                    self.lines.push(String::new());
                    self.current_line = self.lines.len().sub(1);
                    self.pos_in_line = 0;
                }
                Some(s) => {
                    let new_str = if self.pos_in_line == s.chars().count() {
                        String::new()
                    } else {
                        let ss = s.chars().skip(self.pos_in_line).collect();
                        *s = s.chars().take(self.pos_in_line).collect();
                        ss
                    };
                    self.current_line += 1;
                    self.lines.insert(self.current_line, new_str);
                    self.pos_in_line = 0;
                }
            },
            KeyCode::Backspace if !self.lines.is_empty() => {
                if self.pos_in_line == 0 && self.current_line != 0 {
                    let current_line = self.current_line().unwrap().to_string();
                    let previous_line_size = self
                        .lines
                        .get(self.current_line - 1)
                        .map(|l| l.chars().count())
                        .unwrap_or_default();
                    self.lines
                        .get_mut(self.current_line - 1)
                        .unwrap()
                        .push_str(&current_line);
                    self.lines.remove(self.current_line);
                    self.current_line -= 1;
                    self.pos_in_line = previous_line_size;
                } else if self.pos_in_line != 0 {
                    let s = self.lines.get_mut(self.current_line).unwrap();
                    if s.is_empty() {
                        self.lines.pop();
                        self.current_line = self.current_line.saturating_sub(1);
                        self.pos_in_line = self.end_of_current_line();
                    } else {
                        s.remove(s.char_indices().nth(self.pos_in_line - 1).unwrap().0);
                        self.pos_in_line = self.pos_in_line.saturating_sub(1);
                    }
                }
            }
            KeyCode::Up => {
                if self.current_line == 0 {
                    self.pos_in_line = 0;
                }
                self.current_line = self.current_line.saturating_sub(1);
                if self.pos_in_line > self.current_line_size() {
                    self.pos_in_line = self.end_of_current_line();
                }
            }
            KeyCode::Down => {
                if self.current_line + 1 != self.lines.len() {
                    self.current_line += 1;
                    if self.pos_in_line > self.current_line_size() {
                        self.pos_in_line = self.end_of_current_line();
                    }
                }
            }
            KeyCode::Left => {
                self.pos_in_line = self.pos_in_line.saturating_sub(1);
            }
            KeyCode::Right => {
                if let Some(l) = self.current_line()
                    && self.pos_in_line != l.chars().count()
                {
                    self.pos_in_line += 1;
                }
            }
            _ => {
                return false;
            }
        }

        if is_at_end_of_current_line != (self.pos_in_line == self.end_of_current_line()) {
            // we should redraw the cursor style in this case
            self.last_cursor_pos = Position::default();
        }

        true
    }
}

#[async_trait]
impl MouseHandler for TextEdit {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
