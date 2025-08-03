// SPDX-License-Identifier: MIT

use super::shortcut::Shortcut;
use super::style;
use ratatui::style::Stylize;
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

pub struct Header<'a> {
    title: &'a str,
    is_active: bool,
    shortcut: Option<&'a Shortcut>,
}

impl<'a> Header<'a> {
    pub fn new(title: &'a str, is_active: bool, shortcut: Option<&'a Shortcut>) -> Self {
        Self {
            title,
            is_active,
            shortcut,
        }
    }
    pub fn block(&self) -> Block<'a> {
        let border_style = if self.is_active {
            style::ACTIVE_BLOCK_STYLE
        } else {
            style::INACTIVE_BLOCK_STYLE
        };

        let mut b = Block::new()
            .style(style::DEFAULT_STYLE)
            .title(Line::raw(self.title).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(border_style);

        if let Some(s) = self.shortcut {
            let mut l = Vec::new();
            for c in s.current_input_keys() {
                l.push(Span::styled(
                    c.to_string(),
                    border_style.bold().fg(style::HEADER_KEY_SELECTED_FG),
                ));
            }
            for c in s.keys().iter().skip(s.current_input_keys().len()) {
                l.push(Span::styled(c.to_string(), style::HEADER_KEY_FG));
            }
            b = b.title(Line::from(l).right_aligned());
        }

        b
    }
}
