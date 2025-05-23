// SPDX-License-Identifier: MIT

use super::{dialog::DialogTrait, shortcut::SharedData, style};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, Widget},
};

use async_trait::async_trait;
use std::sync::{Arc, RwLock};

struct Shortcut {
    name: String,
    keys: String,
}

pub struct Dialog {
    shortcuts: Vec<Shortcut>,
    should_be_closed: bool,
}

impl Dialog {
    pub fn new(shortcuts: &[Arc<RwLock<SharedData>>]) -> Self {
        let mut s: Vec<Shortcut> = shortcuts
            .iter()
            .map(|s| {
                let d = s.read().unwrap();
                Shortcut {
                    name: d.name.clone(),
                    keys: String::from_iter(d.keys.iter()),
                }
            })
            .collect();

        s.sort_by_key(|s| s.name.clone());

        Self {
            shortcuts: s,
            should_be_closed: false,
        }
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top("Key bindings")
            .title_bottom("Press q or Esc to close")
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        Widget::render(&b, area, buf);

        let items = self
            .shortcuts
            .iter()
            .map(|s| {
                Line::from(vec![
                    Span::styled(format!("{}: ", s.name), Style::new().bold()),
                    Span::raw(s.keys.clone()),
                ])
            })
            .collect::<Vec<Line>>();
        List::new(items).block(b).render(area, buf);
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
            self.should_be_closed = true;
        }
    }

    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn size(&self) -> Size {
        let count: u16 = self.shortcuts.len().try_into().unwrap_or_default();
        Size::new(70, count + 2)
    }
}
