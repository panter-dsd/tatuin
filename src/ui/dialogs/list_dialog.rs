// SPDX-License-Identifier: MIT

use std::fmt::Display;

use super::DialogTrait;
use crate::ui::{
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    selectable_list::SelectableList,
    widgets::WidgetTrait,
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

const FOOTER: &str = "Use j/k (up/down) for moving and Enter for applying";

pub struct Dialog<T> {
    title: String,
    width: u16,
    items: SelectableList<T>,
    should_be_closed: bool,
    selected_item: Option<T>,
}

impl<T> Dialog<T>
where
    T: Display + Clone,
{
    pub fn new(items: &[T], current: &str) -> Self {
        let title = format!("Current value: {current}");
        let title_width = Text::from(title.as_str()).width() as u16;
        let footer_width = Text::from(FOOTER).width() as u16;
        Self {
            title,
            width: std::cmp::max(title_width, footer_width),
            items: SelectableList::new(items.to_vec(), Some(0)),
            should_be_closed: false,
            selected_item: None,
        }
    }

    pub fn selected(&self) -> &Option<T> {
        &self.selected_item
    }
}

#[async_trait]
impl<T> WidgetTrait for Dialog<T>
where
    T: Display + Clone + Send + Sync + 'static,
{
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top(self.title.as_str())
            .title_bottom(FOOTER)
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        Widget::render(&b, area, buf);

        self.items
            .render("", |s| ListItem::from(s.to_string()), b.inner(area), buf);
    }

    fn size(&self) -> Size {
        let mut s = self.items.size();
        s.height += 2;
        s.width = std::cmp::max(s.width, self.width) + 2;
        s
    }
}

#[async_trait]
impl<T> DialogTrait for Dialog<T>
where
    T: Display + Clone + Send + Sync + 'static,
{
    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl<T> MouseHandler for Dialog<T>
where
    T: Send,
{
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}

#[async_trait]
impl<T> KeyboardHandler for Dialog<T>
where
    T: Send + Sync + Clone + 'static,
{
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_be_closed = true;
            }
            KeyCode::Char('j') | KeyCode::Down => self.items.select_next().await,
            KeyCode::Char('k') | KeyCode::Up => self.items.select_previous().await,
            KeyCode::Char('g') | KeyCode::Home => self.items.select_first().await,
            KeyCode::Char('G') | KeyCode::End => self.items.select_last().await,
            KeyCode::Enter => {
                self.should_be_closed = true;
                if let Some(s) = self.items.selected() {
                    self.selected_item = Some(s.clone());
                }
            }
            _ => {}
        }

        true
    }
}
