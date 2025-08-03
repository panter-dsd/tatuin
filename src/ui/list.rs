// SPDX-License-Identifier: MIT

use super::header;
use super::shortcut::Shortcut;
use super::style;
use ratatui::widgets::{Block, HighlightSpacing, List as ListWidget, ListItem};

pub struct List<'a, T> {
    items: &'a [T],
    is_active: bool,
    title: Option<&'a str>,
    shortcut: Option<&'a Shortcut>,
}

impl<'a, T> List<'a, T>
where
    T: Clone,
    T: Into<ListItem<'a>>,
{
    pub fn new(items: &'a [T], is_active: bool) -> Self {
        Self {
            items,
            is_active,
            title: None,
            shortcut: None,
        }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn shortcut(mut self, s: &'a Shortcut) -> Self {
        self.shortcut.replace(s);
        self
    }

    pub fn widget(&self) -> ListWidget<'a> {
        let block = if let Some(t) = self.title {
            header::Header::new(t, self.is_active, self.shortcut).block()
        } else {
            Block::new()
        };

        ListWidget::new(self.items.to_vec())
            .block(block.style(style::DEFAULT_STYLE))
            .style(style::DEFAULT_STYLE)
            .highlight_style(style::SELECTED_ROW_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always)
    }
}
