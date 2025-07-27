// SPDX-License-Identifier: MIT

use std::any::Any;

use super::DialogTrait;
use crate::{
    types::ArcRwLockBlocked,
    ui::{
        keyboard_handler::KeyboardHandler,
        mouse_handler::MouseHandler,
        shortcut::SharedData,
        style,
        widgets::{WidgetState, WidgetStateTrait, WidgetTrait},
    },
};
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect, Size},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, Paragraph, Widget},
};

use async_trait::async_trait;

struct Shortcut {
    name: String,
    keys: String,
}

pub struct Dialog {
    active_block_shortcuts: Vec<Shortcut>,
    global_shortcuts: Vec<Shortcut>,
    should_be_closed: bool,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(Dialog);

fn keys_to_str(keys: &[char]) -> String {
    let mut s = String::new();
    for c in keys {
        if *c == ' ' {
            s.push_str("space ");
        } else {
            s.push(*c)
        }
    }
    s
}

fn shared_data_to_shortcut(s: &ArcRwLockBlocked<SharedData>) -> Shortcut {
    let d = s.read().unwrap();
    Shortcut {
        name: d.name.clone(),
        keys: keys_to_str(&d.keys),
    }
}

impl Dialog {
    pub fn new(
        active_block_shortcuts: &[ArcRwLockBlocked<SharedData>],
        global_shortcuts: &[ArcRwLockBlocked<SharedData>],
    ) -> Self {
        let mut active: Vec<Shortcut> = active_block_shortcuts.iter().map(shared_data_to_shortcut).collect();
        let mut global: Vec<Shortcut> = global_shortcuts.iter().map(shared_data_to_shortcut).collect();

        active.sort_by_key(|s| s.name.clone());
        global.sort_by_key(|s| s.name.clone());

        Self {
            active_block_shortcuts: active,
            global_shortcuts: global,
            should_be_closed: false,
            widget_state: WidgetState::default(),
        }
    }
}

#[async_trait]
impl WidgetTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_alignment(Alignment::Center)
            .title_top("Key bindings")
            .title_bottom("Press q or Esc to close")
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);

        Widget::render(&b, area, buf);

        let mut area = area;
        area.y += 1;
        area.height -= 2;
        area.x += 1;
        area.width -= 2;

        let [active_area, global_area] = Layout::vertical([
            Constraint::Length(self.active_block_shortcuts.len() as u16 + 1),
            Constraint::Fill(1),
        ])
        .areas(area);

        if self.active_block_shortcuts.is_empty() {
            Paragraph::new("There are no shortcut keys in the active panel")
                .alignment(Alignment::Center)
                .style(style::WARNING_TEXT_STYLE)
                .render(active_area, buf);
        } else {
            let active_items = self
                .active_block_shortcuts
                .iter()
                .map(|s| {
                    Line::from(vec![
                        Span::styled(format!("{}: ", s.name), Style::new().bold()),
                        Span::raw(s.keys.clone()),
                    ])
                })
                .collect::<Vec<Line>>();
            let active_block = Block::default()
                .title_alignment(Alignment::Center)
                .title_top("Active block");
            List::new(active_items).block(active_block).render(active_area, buf);
        }

        let global_items = self
            .global_shortcuts
            .iter()
            .map(|s| {
                Line::from(vec![
                    Span::styled(format!("{}: ", s.name), Style::new().bold()),
                    Span::raw(s.keys.clone()),
                ])
            })
            .collect::<Vec<Line>>();
        let global_block = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top("Global shortcuts");
        List::new(global_items).block(global_block).render(global_area, buf);
    }

    fn size(&self) -> Size {
        let count = (self.active_block_shortcuts.len() + self.global_shortcuts.len()) as u16;
        Size::new(70, count + 2/*head_tail*/ * 2 /*subheads*/)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
            self.should_be_closed = true;
            return true;
        }

        false
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
