// SPDX-License-Identifier: MIT

use crate::state::StateSettings;

use super::dialog::DialogTrait;
use super::mouse_handler::MouseHandler;
use super::selectable_list::SelectableList;
use super::{AppBlockWidget, style};
use async_trait::async_trait;
use crossterm::event::MouseEvent;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::widgets::{Block, Borders, ListItem, Widget};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Dialog {
    states: SelectableList<String>,
    settings: Arc<RwLock<Box<dyn StateSettings>>>,
    should_be_closed: bool,
    selected_state: Option<String>,
}

impl Dialog {
    pub async fn new(settings: &Arc<RwLock<Box<dyn StateSettings>>>) -> Self {
        Self {
            states: SelectableList::new(settings.read().await.states().to_vec(), Some(0)),
            settings: settings.clone(),
            should_be_closed: false,
            selected_state: None,
        }
    }

    pub fn selected_state(&self) -> &Option<String> {
        &self.selected_state
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top("States")
            .title_bottom("Use j/k (up/down) for moving, d for deleting and Enter for applying")
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        Widget::render(&b, area, buf);

        self.states
            .render("", |s| ListItem::from(s.as_str()), b.inner(area), buf);
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_be_closed = true;
            }
            KeyCode::Char('j') | KeyCode::Down => self.states.select_next().await,
            KeyCode::Char('k') | KeyCode::Up => self.states.select_previous().await,
            KeyCode::Char('g') | KeyCode::Home => self.states.select_first().await,
            KeyCode::Char('G') | KeyCode::End => self.states.select_last().await,
            KeyCode::Char('d') => {
                if let Some(s) = self.states.selected() {
                    let _ = self.settings.write().await.remove(s);
                    self.states.set_items(self.settings.read().await.states());
                }
            }
            KeyCode::Enter => {
                self.should_be_closed = true;
                if let Some(s) = self.states.selected() {
                    self.selected_state = Some(s.clone());
                }
            }
            _ => {}
        }
    }

    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn size(&self) -> Size {
        Size::new(70, 10)
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
