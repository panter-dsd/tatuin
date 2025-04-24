use crate::filter::{Due, Filter, FilterState};

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::symbols;
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, StatefulWidget, Widget,
};

use crate::ui::{ACTIVE_BLOCK_STYLE, INACTIVE_BLOCK_STYLE, NORMAL_ROW_BG, SELECTED_STYLE};

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
}

impl FilterWidget {
    pub fn new(f: Filter) -> Self {
        Self {
            is_active: false,
            current_block: FilterBlock::State,
            filter: f,
            filter_state_state: ListState::default(),
            filter_due_state: ListState::default(),
        }
    }

    pub fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
        if !is_active {
            self.current_block = FilterBlock::State;
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

    fn block_style(&self, b: FilterBlock) -> Style {
        if self.is_active && self.current_block == b {
            return ACTIVE_BLOCK_STYLE;
        }

        INACTIVE_BLOCK_STYLE
    }

    fn render_filter_state(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw("Task state").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(self.block_style(FilterBlock::State))
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = vec![
            ListItem::from(filter_element_to_text(
                FilterState::Completed,
                &self.filter.states,
            )),
            ListItem::from(filter_element_to_text(
                FilterState::Uncompleted,
                &self.filter.states,
            )),
            ListItem::from(filter_element_to_text(
                FilterState::InProgress,
                &self.filter.states,
            )),
            ListItem::from(filter_element_to_text(
                FilterState::Unknown,
                &self.filter.states,
            )),
        ];

        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.filter_state_state);
    }

    fn render_filter_due(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw("Task due").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(self.block_style(FilterBlock::Due))
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = vec![
            ListItem::from(filter_element_to_text(Due::Overdue, &self.filter.due)),
            ListItem::from(filter_element_to_text(Due::Today, &self.filter.due)),
            ListItem::from(filter_element_to_text(Due::NoDate, &self.filter.due)),
            ListItem::from(filter_element_to_text(Due::Future, &self.filter.due)),
        ];

        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.filter_due_state);
    }
}

impl Widget for &mut FilterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [filter_state_area, filter_due_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

        self.render_filter_state(filter_state_area, buf);
        self.render_filter_due(filter_due_area, buf);
    }
}

fn filter_element_to_text<T: PartialEq + std::fmt::Display>(e: T, v: &[T]) -> String {
    format!("[{}] {}", if v.contains(&e) { "x" } else { " " }, e)
}
