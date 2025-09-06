// SPDX-License-Identifier: MIT

use super::{WidgetState, WidgetStateTrait, WidgetTrait};
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
use std::any::Any;

pub struct HyperlinkWidget {
    pos: Position,
    area: Rect,
    text: String,
    url: String,
    style: Option<Style>,
    is_under_mouse: bool,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(HyperlinkWidget);

impl HyperlinkWidget {
    pub fn new(text: &str, url: &str) -> Self {
        Self {
            pos: Position::default(),
            area: Rect::default(),
            text: text.to_string(),
            url: url.to_string(),
            style: None,
            is_under_mouse: false,
            widget_state: WidgetState::default(),
        }
    }
}

#[async_trait]
impl WidgetTrait for HyperlinkWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let fg = if self.is_under_mouse {
            style::url_under_mouse_color()
        } else {
            style::url_color()
        };

        self.area = Rect {
            x: self.pos.x,
            y: self.pos.y,
            width: std::cmp::min(area.width, Text::from(self.text.as_str()).width() as u16),
            height: 1,
        };

        let mut style = self.style.unwrap_or_default().underlined();
        if style.fg.is_none() || self.is_under_mouse {
            style = style.fg(fg);
        }
        Paragraph::new(self.text.as_str())
            .wrap(Wrap { trim: false })
            .style(style)
            .render(self.area, buf);
    }

    fn size(&self) -> Size {
        Size::new(Text::from(self.text.as_str()).width() as u16, 1)
    }

    fn set_pos(&mut self, pos: Position) {
        self.pos = pos
    }

    fn style(&self) -> Style {
        self.style.unwrap_or_default()
    }

    fn set_style(&mut self, style: Style) {
        self.style = Some(style)
    }

    fn as_any(&self) -> &dyn Any {
        self
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

        if let MouseEventKind::Up(button) = ev.kind
            && button == MouseButton::Left
            && self.is_under_mouse
        {
            if let Err(e) = crate::utils::open_url(&self.url) {
                tracing::error!(target:"hyperlink_widget", error=?e, url=&self.url, "Open url");
            }
        }
    }
}
