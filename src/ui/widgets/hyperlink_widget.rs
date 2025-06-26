// SPDX-License-Identifier: MIT

use super::WidgetTrait;
use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Style, Stylize},
    text::Text,
    widgets::{Paragraph, Widget, Wrap},
};
use std::process::Command;

pub struct HyperlinkWidget {
    pos: Position,
    area: Rect,
    text: String,
    url: String,
    is_under_mouse: bool,
}

impl HyperlinkWidget {
    pub fn new(text: &str, url: &str) -> Self {
        Self {
            pos: Position::default(),
            area: Rect::default(),
            text: text.to_string(),
            url: url.to_string(),
            is_under_mouse: false,
        }
    }

    pub fn size(&self) -> Size {
        Size::new(Text::from(self.text.as_str()).width() as u16, 1)
    }
}

#[async_trait]
impl WidgetTrait for HyperlinkWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let fg = if self.is_under_mouse {
            style::URL_UNDER_MOUSE_COLOR
        } else {
            style::URL_COLOR
        };

        self.area = Rect {
            x: self.pos.x,
            y: self.pos.y,
            width: std::cmp::min(area.width, Text::from(self.text.as_str()).width() as u16),
            height: 1,
        };

        Paragraph::new(self.text.as_str())
            .wrap(Wrap { trim: false })
            .style(Style::new().underlined().fg(fg))
            .render(self.area, buf);
    }

    fn size(&self) -> Size {
        Size::new(Text::from(self.text.as_str()).width() as u16, 1)
    }

    fn set_pos(&mut self, pos: Position) {
        self.pos = pos
    }
}

#[async_trait]
impl KeyboardHandler for HyperlinkWidget {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

#[async_trait]
impl MouseHandler for HyperlinkWidget {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        let position = Position::new(ev.column, ev.row);
        self.is_under_mouse = self.area.contains(position);

        if let MouseEventKind::Up(button) = ev.kind {
            if button == MouseButton::Left && self.is_under_mouse {
                // Call the `open` command
                let status = Command::new("open")
                    .arg(&self.url)
                    .status()
                    .expect("Failed to execute command");

                // Check if the command was successful
                if !status.success() {
                    eprintln!("Failed to open {}", self.url);
                }
            }
        }
    }
}
