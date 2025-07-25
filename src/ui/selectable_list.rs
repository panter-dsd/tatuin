// SPDX-License-Identifier: MIT

use super::{
    AppBlockWidget, keyboard_handler::KeyboardHandler, list, mouse_handler::MouseHandler, shortcut::Shortcut,
    widgets::WidgetTrait,
};
use crate::state::{State, StatefulObject};
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    widgets::{ListItem, ListState, StatefulWidget},
};
use std::{
    any::Any,
    slice::{Iter, IterMut},
};

const DEFAULT_WIDTH: u16 = 10;

pub struct SelectableList<T> {
    items: Vec<T>,
    state: ListState,
    add_all_item: bool,
    shortcut: Option<Shortcut>,
    is_active: bool,
    show_count_in_title: bool,

    width: u16,
}

#[async_trait]
impl<T> AppBlockWidget for SelectableList<T>
where
    T: Send + Sync + 'static,
{
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        if let Some(s) = &mut self.shortcut {
            vec![s]
        } else {
            Vec::new()
        }
    }

    async fn select_next(&mut self) {
        self.state.select_next();
    }

    async fn select_previous(&mut self) {
        self.state.select_previous();
    }

    async fn select_first(&mut self) {
        self.state.select_first();
    }

    async fn select_last(&mut self) {
        self.state.select_last();
    }
}

#[async_trait]
impl<T> WidgetTrait for SelectableList<T>
where
    T: Send + Sync + 'static,
{
    async fn render(&mut self, _area: Rect, _buf: &mut Buffer) {
        panic!("Don't use this method!")
    }

    fn size(&self) -> Size {
        Size::new(self.width, self.items.len() as u16)
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl<T> MouseHandler for SelectableList<T>
where
    T: Send + Sync,
{
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}

#[async_trait]
impl<T> KeyboardHandler for SelectableList<T>
where
    T: Send,
{
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

impl<T> SelectableList<T> {
    pub fn new(items: Vec<T>, selected: Option<usize>) -> Self {
        Self {
            items,
            state: ListState::default().with_selected(selected),
            add_all_item: false,
            shortcut: None,
            is_active: false,
            show_count_in_title: true,
            width: DEFAULT_WIDTH, // will be recalculated after the first render
        }
    }

    pub fn add_all_item(mut self) -> Self {
        self.add_all_item = true;
        self.state.select_first();
        self
    }

    pub fn shortcut(mut self, s: Shortcut) -> Self {
        self.shortcut = Some(s);
        self
    }

    pub fn add_item(&mut self, item: T) {
        self.items.push(item);
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items
    }

    pub fn set_state(&mut self, state: ListState) {
        self.state = state
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.items.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn selected(&self) -> Option<&T> {
        if self.state.selected().is_some() && !self.items.is_empty() {
            let idx = std::cmp::min(
                self.state.selected().unwrap_or_default(),
                if self.add_all_item {
                    self.items.len()
                } else {
                    self.items.len() - 1
                },
            );
            if self.add_all_item && idx == 0 {
                return None;
            }
            let t = &self.items[if self.add_all_item { idx - 1 } else { idx }];
            Some(t)
        } else {
            None
        }
    }

    pub fn render(&mut self, title: &str, f: impl Fn(&T) -> ListItem, area: Rect, buf: &mut Buffer) {
        let mut items = self.items.iter().map(f).collect::<Vec<ListItem>>();
        if self.add_all_item {
            items.insert(0, ListItem::from("All"));
        }

        self.width = items
            .iter()
            .map(|item| item.width())
            .max()
            .unwrap_or(DEFAULT_WIDTH as usize) as u16;

        let mut l = list::List::new(&items, self.is_active);
        if let Some(s) = &self.shortcut {
            l = l.shortcut(s);
        }

        let header_title;
        if !title.is_empty() {
            header_title = if self.show_count_in_title {
                format!("{title} ({})", items.len())
            } else {
                title.to_string()
            };
            l = l.title(header_title.as_str());
        }

        StatefulWidget::render(l.widget(), area, buf, &mut self.state);
    }
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        SelectableList::new(Vec::new(), None)
    }
}

const STATE_KEY: &str = "selected_item_index";

impl<T> StatefulObject for SelectableList<T> {
    fn save(&self) -> State {
        State::from([(
            STATE_KEY.to_string(),
            self.state.selected().unwrap_or_default().to_string(),
        )])
    }

    fn restore(&mut self, state: State) {
        if let Some(idx) = state.get(STATE_KEY) {
            if let Ok(idx) = idx.parse::<usize>() {
                self.state.select(Some(idx));
            }
        }
    }
}
