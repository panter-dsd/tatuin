use super::mouse_handler::MouseHandler;
use super::style;
use async_trait::async_trait;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Style, Stylize},
    text::Text,
    widgets::{Paragraph, Widget, Wrap},
};
use std::process::Command;

pub struct HyperlinkWidget {
    area: Rect,
    text: String,
    url: String,
    is_under_mouse: bool,
}

impl HyperlinkWidget {
    pub fn new(text: &str, url: &str) -> Self {
        Self {
            area: Rect::default(),
            text: text.to_string(),
            url: url.to_string(),
            is_under_mouse: false,
        }
    }

    pub fn set_pos(&mut self, area: Rect, pos: Position) {
        self.area = Rect::new(
            pos.x,
            pos.y,
            std::cmp::min(area.width, Text::from(self.text.as_str()).width() as u16),
            1,
        )
    }

    pub fn size(&self) -> Size {
        Size::new(Text::from(self.text.as_str()).width() as u16, 1)
    }

    pub fn render(&self, buf: &mut Buffer) {
        let fg = if self.is_under_mouse {
            style::URL_UNDER_MOUSE_COLOR
        } else {
            style::URL_COLOR
        };

        Paragraph::new(self.text.as_str())
            .wrap(Wrap { trim: false })
            .style(Style::new().underlined().fg(fg))
            .render(self.area, buf);
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
