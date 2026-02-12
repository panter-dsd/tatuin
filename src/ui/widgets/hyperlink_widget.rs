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
    widgets::{Clear, Paragraph, Widget, Wrap},
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

    fn tool_tip_rect(&self, area: Rect) -> Rect {
        let text_width = Text::from(self.url.as_str()).width() as u16;
        let mut r = Rect {
            x: self.area.x,
            y: self.area.y - 1,
            width: (area.width - self.area.x).min(text_width),
            height: 1,
        };

        if r.width < text_width {
            r.x = r.x.saturating_sub(text_width - r.width);
            r.width = area.width - r.x;
        }
        r
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
            width: std::cmp::min(
                area.width.saturating_sub(self.pos.x),
                Text::from(self.text.as_str()).width() as u16,
            ),
            height: 1,
        };

        if self.area.width == 0 {
            return; // there is no place for the widget
        }

        let mut style = self.style.unwrap_or_default().underlined();
        if style.fg.is_none() || self.is_under_mouse {
            style = style.fg(fg);
        }
        Paragraph::new(self.text.as_str())
            .wrap(Wrap { trim: false })
            .style(style)
            .render(self.area, buf);
        if self.is_under_mouse {
            let r = self.tool_tip_rect(area);
            Clear {}.render(r, buf);
            Paragraph::new(self.url.as_str())
                .style(style::url_hover_hint_style())
                .render(r, buf);
        }
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
            && let Err(e) = tatuin_core::utils::open_url(&self.url)
        {
            tracing::error!(target:"hyperlink_widget", error=?e, url=&self.url, "Open url");
        }
    }
}
