// SPDX-License-Identifier: MIT

use crate::{
    filter::{Due, Filter, FilterState},
    state::StatefulObject,
    types::ArcRwLock,
};
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use std::{any::Any, sync::Arc};
use tokio::sync::RwLock;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect, Size},
    widgets::{ListItem, ListState, StatefulWidget, Widget},
};

use super::{
    AppBlockWidget, header, keyboard_handler::KeyboardHandler, list, mouse_handler::MouseHandler, shortcut::Shortcut,
    widgets::State, widgets::StateTrait, widgets::WidgetTrait,
};

const POSSIBLE_STATES: [FilterState; 4] = [
    FilterState::Completed,
    FilterState::Uncompleted,
    FilterState::InProgress,
    FilterState::Unknown,
];

const POSSIBLE_DUE: [Due; 4] = [Due::NoDate, Due::Overdue, Due::Today, Due::Future];

#[derive(Eq, PartialEq)]
enum FilterBlock {
    State,
    Due,
}

pub struct FilterWidget {
    current_block: FilterBlock,
    filter: Filter,
    filter_state_state: ListState,
    filter_due_state: ListState,
    state_shortcut: Shortcut,
    due_shortcut: Shortcut,
    state: State,
}

impl StateTrait for FilterWidget {
    fn is_active(&self) -> bool {
        self.state.is_active()
    }

    fn set_active(&mut self, is_active: bool) {
        self.state.set_active(is_active);
    }

    fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }

    fn set_enabled(&mut self, is_enabled: bool) {
        self.state.set_enabled(is_enabled);
    }
}

#[async_trait]
impl AppBlockWidget for FilterWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        vec![&mut self.state_shortcut, &mut self.due_shortcut]
    }

    async fn select_next(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_next(),
            FilterBlock::Due => self.filter_due_state.select_next(),
        }
    }

    async fn select_previous(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_previous(),
            FilterBlock::Due => self.filter_due_state.select_previous(),
        }
    }

    async fn select_first(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_first(),
            FilterBlock::Due => self.filter_due_state.select_first(),
        }
    }

    async fn select_last(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_last(),
            FilterBlock::Due => self.filter_due_state.select_last(),
        }
    }
}

#[async_trait]
impl MouseHandler for FilterWidget {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}

#[async_trait]
impl KeyboardHandler for FilterWidget {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

impl FilterWidget {
    pub fn new(f: Filter) -> ArcRwLock<Self> {
        let s = Arc::new(RwLock::new(Self {
            state: State::default(),
            current_block: FilterBlock::State,
            filter: f,
            filter_state_state: ListState::default(),
            filter_due_state: ListState::default(),
            state_shortcut: Shortcut::new("Activate Filter->State block", &['g', 's']),
            due_shortcut: Shortcut::new("Activate Filter->Due block", &['g', 'd']),
        }));

        tokio::spawn({
            let s = s.clone();
            async move {
                let mut state_rx = s.read().await.state_shortcut.subscribe_to_accepted();
                let mut due_rx = s.read().await.due_shortcut.subscribe_to_accepted();
                loop {
                    let block = tokio::select! {
                        _ = state_rx.recv() => FilterBlock::State,
                        _ = due_rx.recv() => FilterBlock::Due,
                    };
                    s.write().await.current_block = block;
                }
            }
        });
        s
    }

    pub fn set_active(&mut self, is_active: bool, backward: bool) {
        StateTrait::set_active(self, is_active);
        if is_active {
            self.current_block = if backward { FilterBlock::Due } else { FilterBlock::State };
        }
    }

    pub fn change_check_state(&mut self) {
        match self.current_block {
            FilterBlock::State => {
                if let Some(idx) = self.filter_state_state.selected() {
                    let st = &POSSIBLE_STATES[idx];
                    if let Some(i) = self.filter.states.iter().position(|s| s == st) {
                        self.filter.states.remove(i);
                    } else {
                        self.filter.states.push(st.clone());
                    }
                }
            }
            FilterBlock::Due => {
                if let Some(idx) = self.filter_due_state.selected() {
                    let due = &POSSIBLE_DUE[idx];
                    if let Some(i) = self.filter.due.iter().position(|d| d == due) {
                        self.filter.due.remove(i);
                    } else {
                        self.filter.due.push(due.clone());
                    }
                }
            }
        }
    }

    pub fn filter(&self) -> Filter {
        self.filter.clone()
    }

    pub fn next_block(&mut self) -> bool {
        match self.current_block {
            FilterBlock::State => {
                self.current_block = FilterBlock::Due;
                true
            }
            FilterBlock::Due => false,
        }
    }

    pub fn previous_block(&mut self) -> bool {
        match self.current_block {
            FilterBlock::State => false,
            FilterBlock::Due => {
                self.current_block = FilterBlock::State;
                true
            }
        }
    }

    fn render_filter_state(&mut self, area: Rect, buf: &mut Buffer) {
        let items = POSSIBLE_STATES
            .iter()
            .map(|s| {
                let t = filter_element_to_text(s.clone(), &self.filter.states);
                ListItem::from(t)
            })
            .collect::<Vec<ListItem>>();

        StatefulWidget::render(
            list::List::new(&items, self.is_active() && self.current_block == FilterBlock::State)
                .title("Task state")
                .shortcut(&self.state_shortcut)
                .widget(),
            area,
            buf,
            &mut self.filter_state_state,
        );
    }

    fn render_filter_due(&mut self, area: Rect, buf: &mut Buffer) {
        let items = POSSIBLE_DUE
            .iter()
            .map(|s| {
                let t = filter_element_to_text(s.clone(), &self.filter.due);
                ListItem::from(t)
            })
            .collect::<Vec<ListItem>>();

        StatefulWidget::render(
            list::List::new(&items, self.is_active() && self.current_block == FilterBlock::Due)
                .title("Task due")
                .shortcut(&self.due_shortcut)
                .widget(),
            area,
            buf,
            &mut self.filter_due_state,
        );
    }
}

#[async_trait]
impl WidgetTrait for FilterWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header_area, body_area] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
        let [filter_state_area, filter_due_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(body_area);

        header::Header::new("Filter", self.is_active(), None)
            .block()
            .render(header_area, buf);
        self.render_filter_state(filter_state_area, buf);
        self.render_filter_due(filter_due_area, buf);
    }

    fn size(&self) -> Size {
        Size { width: 0, height: 6 }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

const STATE_KEY: &str = "filter";

impl StatefulObject for FilterWidget {
    fn save(&self) -> crate::state::State {
        let mut state = crate::state::State::new();

        if let Ok(s) = serde_json::to_string(&self.filter) {
            state.insert(STATE_KEY.to_string(), s);
        }

        state
    }

    fn restore(&mut self, state: crate::state::State) {
        if let Some(s) = state.get(STATE_KEY) {
            if let Ok(f) = serde_json::from_str(s) {
                self.filter = f;
            }
        }
    }
}

fn filter_element_to_text<T: PartialEq + std::fmt::Display>(e: T, v: &[T]) -> String {
    format!("[{}] {}", if v.contains(&e) { "x" } else { " " }, e)
}
