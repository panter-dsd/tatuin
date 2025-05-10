use super::shortcut::Shortcut;
use super::style;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

pub struct Header<'a> {
    title: &'a str,
    is_active: bool,
    shortcut: &'a Option<Shortcut>,
}

impl<'a> Header<'a> {
    pub fn new(title: &'a str, is_active: bool, shortcut: &'a Option<Shortcut>) -> Self {
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
            .title(Line::raw(self.title).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(border_style)
            .bg(style::NORMAL_ROW_BG);

        if let Some(s) = self.shortcut {
            let mut l = Vec::new();
            for c in s.partially_keys() {
                l.push(Span::styled(
                    c.to_string(),
                    Style::default().bold().fg(style::HEADER_KEY_SELECTED_FG),
                ));
            }
            for c in s.keys().iter().skip(s.partially_keys().len()) {
                l.push(Span::styled(c.to_string(), style::HEADER_KEY_FG));
            }
            b = b.title(Line::from(l).right_aligned());
        }

        b
    }
}
