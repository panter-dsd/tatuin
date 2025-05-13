use crate::filter::{Due, Filter, FilterState};
use std::sync::Arc;
use tokio::sync::RwLock;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{ListItem, ListState, StatefulWidget, Widget};

use super::list;
use super::shortcut::Shortcut;
use super::{AppBlockWidget, header};

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
    is_active: bool,
    current_block: FilterBlock,
    filter: Filter,
    filter_state_state: ListState,
    filter_due_state: ListState,
    state_shortcut: Shortcut,
    due_shortcut: Shortcut,
}

impl AppBlockWidget for FilterWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        vec![&mut self.state_shortcut, &mut self.due_shortcut]
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }
}

impl FilterWidget {
    pub fn new(f: Filter) -> Arc<RwLock<Self>> {
        let s = Arc::new(RwLock::new(Self {
            is_active: false,
            current_block: FilterBlock::State,
            filter: f,
            filter_state_state: ListState::default(),
            filter_due_state: ListState::default(),
            state_shortcut: Shortcut::new(&['g', 's']),
            due_shortcut: Shortcut::new(&['g', 'd']),
        }));

        tokio::spawn({
            let s = s.clone();
            async move {
                let mut rx = s.read().await.state_shortcut.subscribe_to_accepted();
                loop {
                    if rx.recv().await.is_err() {
                        return;
                    }

                    s.write().await.current_block = FilterBlock::State;
                }
            }
        });
        tokio::spawn({
            let s = s.clone();
            async move {
                let mut rx = s.read().await.due_shortcut.subscribe_to_accepted();
                loop {
                    if rx.recv().await.is_err() {
                        return;
                    }

                    s.write().await.current_block = FilterBlock::Due;
                }
            }
        });
        s
    }

    pub fn set_active(&mut self, is_active: bool, backward: bool) {
        self.is_active = is_active;
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

    pub fn select_none(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select(None),
            FilterBlock::Due => self.filter_due_state.select(None),
        }
    }

    pub fn select_next(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_next(),
            FilterBlock::Due => self.filter_due_state.select_next(),
        }
    }

    pub fn select_previous(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_previous(),
            FilterBlock::Due => self.filter_due_state.select_previous(),
        }
    }

    pub fn select_first(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_first(),
            FilterBlock::Due => self.filter_due_state.select_first(),
        }
    }

    pub fn select_last(&mut self) {
        match self.current_block {
            FilterBlock::State => self.filter_state_state.select_last(),
            FilterBlock::Due => self.filter_due_state.select_last(),
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
            list::List::new(&items, self.is_active && self.current_block == FilterBlock::State)
                .title("Task state")
                .shortcut(&Some(self.state_shortcut.clone()))
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
            list::List::new(&items, self.is_active && self.current_block == FilterBlock::Due)
                .title("Task due")
                .shortcut(&Some(self.due_shortcut.clone()))
                .widget(),
            area,
            buf,
            &mut self.filter_due_state,
        );
    }
}

impl Widget for &mut FilterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, body_area] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
        let [filter_state_area, filter_due_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(body_area);

        header::Header::new("Filter", self.is_active, &None)
            .block()
            .render(header_area, buf);
        self.render_filter_state(filter_state_area, buf);
        self.render_filter_due(filter_due_area, buf);
    }
}

fn filter_element_to_text<T: PartialEq + std::fmt::Display>(e: T, v: &[T]) -> String {
    format!("[{}] {}", if v.contains(&e) { "x" } else { " " }, e)
}
