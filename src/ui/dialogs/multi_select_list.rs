// SPDX-License-Identifier: MIT

use std::{any::Any, fmt::Display};

use super::DialogTrait;
use crate::ui::{
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    selectable_list::SelectableList,
    widgets::{WidgetState, WidgetStateTrait, WidgetTrait},
    {AppBlockWidget, style},
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    text::Text,
    widgets::{Block, Borders, ListItem, Widget},
};

const FOOTER: &str = "Use j/k (up/down) for moving, Space for selecting and Enter for applying";

pub struct Dialog<T> {
    title: String,
    width: u16,
    items: SelectableList<T>,
    selected: Vec<T>,
    should_be_closed: bool,
    show_top_title: bool,
    show_bottom_title: bool,
    widget_state: WidgetState,
}

impl<T> WidgetStateTrait for Dialog<T> {
    fn is_active(&self) -> bool {
        self.widget_state.is_active()
    }

    fn set_active(&mut self, is_active: bool) {
        self.widget_state.set_active(is_active);
    }

    fn is_enabled(&self) -> bool {
        self.widget_state.is_enabled()
    }

    fn set_enabled(&mut self, is_enabled: bool) {
        self.widget_state.set_enabled(is_enabled);
    }

    fn is_visible(&self) -> bool {
        self.widget_state.is_visible()
    }

    fn set_visible(&mut self, is_visible: bool) {
        self.widget_state.set_visible(is_visible);
    }
}

#[allow(dead_code)]
impl<T> Dialog<T>
where
    T: Display + Clone,
{
    pub fn new(items: &[T]) -> Self {
        let mut s = Self {
            title: "Select one or several items".to_string(),
            width: 0,
            items: SelectableList::new(items.to_vec(), Some(0)),
            selected: Vec::new(),
            should_be_closed: false,
            show_top_title: true,
            show_bottom_title: true,
            widget_state: WidgetState::default(),
        };
        s.calculate_width();
        s
    }

    pub fn show_top_title(mut self, is_show: bool) -> Self {
        self.show_top_title = is_show;
        self.calculate_width();
        self
    }

    pub fn show_bottom_title(mut self, is_show: bool) -> Self {
        self.show_bottom_title = is_show;
        self.calculate_width();
        self
    }

    pub fn set_selected(&mut self, selected: &[T]) {
        self.selected = selected.to_vec()
    }

    pub fn selected(&self) -> Vec<T> {
        self.selected.clone()
    }
}

impl<T> Dialog<T>
where
    T: Display + Clone,
{
    fn calculate_width(&mut self) {
        let mut w = self
            .items
            .iter()
            .map(|item| Text::from(item.to_string()).width())
            .max()
            .unwrap_or_default();
        if self.show_top_title {
            w = w.max(Text::from(self.title.as_str()).width());
        }
        if self.show_bottom_title {
            w = w.max(Text::from(FOOTER).width());
        }
        self.width = w as u16;
    }
}

#[async_trait]
impl<T> WidgetTrait for Dialog<T>
where
    T: Display + Eq + Clone + Send + Sync + 'static,
{
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut b = Block::new()
            .style(self.style())
            .title_alignment(ratatui::layout::Alignment::Center)
            .borders(Borders::ALL)
            .border_style(style::border_color());
        if self.show_top_title {
            b = b.title_top(self.title.as_str());
        }
        if self.show_bottom_title {
            b = b.title_bottom(FOOTER);
        }
        Widget::render(&b, area, buf);

        let inner_area = b.inner(area);
        self.items.render(
            "",
            |s| {
                let t = format!("[{}] {s}", if self.selected.contains(s) { 'x' } else { ' ' });
                ListItem::from(t)
            },
            inner_area,
            buf,
        );
    }

    fn size(&self) -> Size {
        let mut s = self.items.size();
        s.height += 2;
        s.width = self.width + 1/*selector*/ + 2 /*borders*/;
        s
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl<T> DialogTrait for Dialog<T>
where
    T: Display + Eq + Clone + Send + Sync + 'static,
{
    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl<T> MouseHandler for Dialog<T>
where
    T: Send + Sync,
{
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}

#[async_trait]
impl<T> KeyboardHandler for Dialog<T>
where
    T: Display + PartialEq + Send + Sync + Clone + 'static,
{
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_be_closed = true;
            }
            KeyCode::Char('j') | KeyCode::Char('n') | KeyCode::Down => self.items.select_next().await,
            KeyCode::Char('k') | KeyCode::Char('p') | KeyCode::Up => self.items.select_previous().await,
            KeyCode::Char('g') | KeyCode::Home => self.items.select_first().await,
            KeyCode::Char('G') | KeyCode::End => self.items.select_last().await,
            KeyCode::Char('a') => {
                self.selected = self.items.iter().cloned().collect();
            }
            KeyCode::Char('c') => self.selected.clear(),
            KeyCode::Char(' ') => {
                if let Some(v) = self.items.selected() {
                    if let Some(idx) = self.selected.iter().position(|s| s == v) {
                        self.selected.remove(idx);
                    } else {
                        self.selected.push(v.clone());
                    }
                }
            }
            KeyCode::Enter => {
                self.should_be_closed = true;
            }
            _ => {}
        }

        true
    }
}
