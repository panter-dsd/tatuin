// SPDX-License-Identifier: MIT

use std::{any::Any, fmt::Display, sync::Arc};

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

const FOOTER: &str = "Use j/k (up/down) for moving and Enter for applying";

pub struct Dialog<T> {
    title: String,
    width: u16,
    items: SelectableList<T>,
    custom_widgets: Vec<Arc<dyn WidgetTrait>>,
    should_be_closed: bool,
    item_has_chosen: bool,
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

impl<T> Dialog<T>
where
    T: Display + Clone,
{
    pub fn new(items: &[T], current: &str) -> Self {
        let mut s = Self {
            title: format!("Current value: {current}"),
            width: 0,
            items: SelectableList::new(
                items.to_vec(),
                items.iter().position(|s| s.to_string() == current).or(Some(0)),
            ),
            custom_widgets: Vec::new(),
            should_be_closed: false,
            item_has_chosen: false,
            show_top_title: true,
            show_bottom_title: true,
            widget_state: WidgetState::default(),
        };
        s.calculate_width();
        s
    }

    pub fn set_current_item(&mut self, current: &str) {
        let idx = self.items.iter().position(|s| s.to_string() == current).or(Some(0));
        self.items.set_selected_index(idx);
    }

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

    pub fn selected(&self) -> Option<&T> {
        if self.item_has_chosen {
            self.items.selected()
        } else {
            None
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        if self.item_has_chosen {
            self.items.selected_index()
        } else {
            None
        }
    }

    pub fn selected_custom_widget(&self) -> Option<Arc<dyn WidgetTrait>> {
        self.current_custom_widget_index()
            .map(|i| self.custom_widgets[i].clone())
    }

    pub fn add_custom_widget(&mut self, item: T, w: Arc<dyn WidgetTrait>) {
        self.items.add_item(item);
        self.custom_widgets.push(w);
        self.calculate_width();
    }

    pub fn custom_widgets(&self) -> Vec<Arc<dyn WidgetTrait>> {
        self.custom_widgets.clone()
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
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        if self.show_top_title {
            b = b.title_top(self.title.as_str());
        }
        if self.show_bottom_title {
            b = b.title_bottom(FOOTER);
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

            Arc::get_mut(w).unwrap().render(rect, buf).await;
        }
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
            if Arc::get_mut(w).unwrap().handle_key(key).await {
                return true;
            }
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_be_closed = true;
            }
            KeyCode::Char('j') | KeyCode::Char('n') | KeyCode::Down => self.items.select_next().await,
            KeyCode::Char('k') | KeyCode::Char('p') | KeyCode::Up => self.items.select_previous().await,
            KeyCode::Char('g') | KeyCode::Home => self.items.select_first().await,
            KeyCode::Char('G') | KeyCode::End => self.items.select_last().await,
            KeyCode::Enter => {
                self.should_be_closed = true;
                self.item_has_chosen = true;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(idx) = self.current_custom_widget_index() {
                    Arc::get_mut(&mut self.custom_widgets[idx]).unwrap().set_active(true);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(idx) = self.current_custom_widget_index() {
                    Arc::get_mut(&mut self.custom_widgets[idx]).unwrap().set_active(false);
                }
            }
            _ => {}
        }

        true
    }
}
