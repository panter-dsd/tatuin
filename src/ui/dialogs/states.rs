// SPDX-License-Identifier: MIT

use std::any::Any;

use crate::{state::StateSettings, types::ArcRwLock};

use super::DialogTrait;
use crate::ui::{
    AppBlockWidget,
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    selectable_list::SelectableList,
    style,
    widgets::{WidgetState, WidgetStateTrait, WidgetTrait},
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    widgets::{Block, Borders, ListItem, Widget},
};

pub struct Dialog {
    states: SelectableList<String>,
    settings: ArcRwLock<Box<dyn StateSettings>>,
    should_be_closed: bool,
    selected_state: Option<String>,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(Dialog);

impl Dialog {
    pub async fn new(settings: &ArcRwLock<Box<dyn StateSettings>>) -> Self {
        Self {
            states: SelectableList::new(settings.read().await.states().to_vec(), Some(0)),
            settings: settings.clone(),
            should_be_closed: false,
            selected_state: None,
            widget_state: WidgetState::default(),
        }
    }

    pub fn selected_state(&self) -> &Option<String> {
        &self.selected_state
    }
}

#[async_trait]
impl WidgetTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top("States")
            .title_bottom("Use j/k (up/down) for moving, d for deleting and Enter for applying")
            .borders(Borders::ALL)
            .border_style(style::border_color());
        Widget::render(&b, area, buf);

        self.states
            .render("", |s| ListItem::from(s.as_str()), b.inner(area), buf);
    }

    fn size(&self) -> Size {
        Size::new(70, 10)
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
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
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
            _ => {
                return false;
            }
        }

        true
    }
}
