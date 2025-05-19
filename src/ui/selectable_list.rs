// SPDX-License-Identifier: MIT

use super::AppBlockWidget;
use super::list;
use super::shortcut::Shortcut;
use crate::state::{State, StatefulObject};
use async_trait::async_trait;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{ListItem, ListState, StatefulWidget};
use std::slice::{Iter, IterMut};

pub struct SelectableList<T> {
    items: Vec<T>,
    state: ListState,
    add_all_item: bool,
    shortcut: Option<Shortcut>,
    is_active: bool,
}

#[async_trait]
impl<T> AppBlockWidget for SelectableList<T>
where
    T: Send,
{
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        if let Some(s) = &mut self.shortcut {
            vec![s]
        } else {
            Vec::new()
        }
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
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

impl<T> SelectableList<T> {
    pub fn new(items: Vec<T>, selected: Option<usize>) -> Self {
        Self {
            items,
            state: ListState::default().with_selected(selected),
            add_all_item: false,
            shortcut: None,
            is_active: false,
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

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items
    }

    pub fn set_state(&mut self, state: ListState) {
        self.state = state
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> Iter<T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.items.iter_mut()
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

    pub fn state(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn render(&mut self, title: &str, f: impl Fn(&T) -> ListItem, area: Rect, buf: &mut Buffer) {
        let mut items = self.items.iter().map(f).collect::<Vec<ListItem>>();
        if self.add_all_item {
            items.insert(0, ListItem::from("All"));
        }

        StatefulWidget::render(
            list::List::new(&items, self.is_active)
                .title(format!("{title} ({})", items.len()).as_str())
                .shortcut(&self.shortcut)
                .widget(),
            area,
            buf,
            &mut self.state,
        );
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
