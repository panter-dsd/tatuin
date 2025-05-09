use super::shortcut::Shortcut;
use super::style;
use ratatui::style::Stylize;
use ratatui::symbols;
use ratatui::text::Line;
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
            b = b.title(Line::raw(s.text()).right_aligned());
        }

        b
    }
}
