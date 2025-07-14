// SPDX-License-Identifier: MIT

use std::{any::Any, fmt::Display};

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
    custom_widgets: Vec<Box<dyn WidgetTrait>>,
    should_be_closed: bool,
    selected_item: Option<T>,
    show_top_title: bool,
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
            items: SelectableList::new(
                items.to_vec(),
                items.iter().position(|s| s.to_string() == current).or(Some(0)),
            ),
            custom_widgets: Vec::new(),
            should_be_closed: false,
            selected_item: None,
            show_top_title: true,
        }
    }

    pub fn show_top_title(mut self, is_show: bool) -> Self {
        self.show_top_title = is_show;
        self
    }

    pub fn selected(&self) -> &Option<T> {
        &self.selected_item
    }

    pub fn selected_custom_widget(&self) -> Option<&dyn WidgetTrait> {
        self.current_custom_widget_index()
            .map(|i| self.custom_widgets[i].as_ref() as &dyn WidgetTrait)
    }

    pub fn add_custom_widget(&mut self, item: T, w: Box<dyn WidgetTrait>) {
        self.items.add_item(item);
        self.custom_widgets.push(w);
    }

    fn current_custom_widget_index(&self) -> Option<usize> {
        if let Some(idx) = self.items.selected_index() {
            if idx < self.items.len() && idx >= self.items.len() - self.custom_widgets.len() {
                let widgets_count = self.custom_widgets.len();
                return Some(idx - (self.items.len() - widgets_count));
            }
        }

        None
    }
}

#[async_trait]
impl<T> WidgetTrait for Dialog<T>
where
    T: Display + Clone + Send + Sync + 'static,
{
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut b = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(FOOTER)
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        if self.show_top_title {
            b = b.title_top(self.title.as_str());
        }
        Widget::render(&b, area, buf);

        self.items.set_active(
            !self
                .current_custom_widget_index()
                .is_some_and(|idx| self.custom_widgets[idx].is_active()),
        );

        let inner_area = b.inner(area);
        self.items
            .render("", |s| ListItem::from(s.to_string()), inner_area, buf);

        let custom_widgets_len = self.custom_widgets.len() as u16;
        let custom_widgets_y = inner_area.y + inner_area.height - custom_widgets_len;
        for (i, w) in self.custom_widgets.iter_mut().enumerate() {
            let item_index = self.items.len() - custom_widgets_len as usize + i;
            let text_width = Text::from(self.items.iter().nth(item_index).unwrap().to_string()).width() as u16;
            let rect = Rect::new(
                inner_area.x + text_width + 2,
                custom_widgets_y + i as u16,
                inner_area.width,
                1,
            );

            w.render(rect, buf).await;
        }
    }

    fn size(&self) -> Size {
        let mut s = self.items.size();
        s.height += 2;
        s.width = std::cmp::max(s.width, self.width) + 2;
        s
    }

    fn as_any(&self) -> &dyn Any {
        self
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
    T: Display + Send + Sync + Clone + 'static,
{
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        for w in self.custom_widgets.iter_mut() {
            if w.handle_key(key).await {
                return true;
            }
        }

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
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(idx) = self.current_custom_widget_index() {
                    self.custom_widgets[idx].set_active(true);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(idx) = self.current_custom_widget_index() {
                    self.custom_widgets[idx].set_active(false);
                }
            }
            _ => {}
        }

        true
    }
}
