use ratatui::widgets::ListState;
use std::slice::{Iter, IterMut};

pub struct SelectableList<T> {
    items: Vec<T>,
    state: ListState,
}

impl<T> SelectableList<T> {
    pub fn new(v: Vec<T>, selected: Option<usize>) -> Self {
        Self {
            items: v,
            state: ListState::default().with_selected(selected),
        }
    }

    pub fn state(&mut self) -> &mut ListState {
        &mut self.state
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
            let selected_idx = std::cmp::min(
                self.state.selected().unwrap_or_default(),
                self.items.len() - 1,
            );
            let t = &self.items[selected_idx];
            Some(t)
        } else {
            None
        }
    }

    pub fn selected_idx(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn item(&self, idx: usize) -> &T {
        &self.items[idx]
    }

    pub fn select_none(&mut self) {
        self.state.select(None)
    }

    pub fn select_next(&mut self) {
        self.state.select_next()
    }

    pub fn select_previous(&mut self) {
        self.state.select_previous()
    }

    pub fn select_first(&mut self) {
        self.state.select_first()
    }

    pub fn select_last(&mut self) {
        self.state.select_last()
    }
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: ListState::default(),
        }
    }
}
