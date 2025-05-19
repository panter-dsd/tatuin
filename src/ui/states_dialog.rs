use super::selectable_list::SelectableList;
use super::style;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, ListItem, Widget};

pub struct StatesDialog {
    states: SelectableList<String>,
    should_be_closed: bool,
}

impl StatesDialog {
    pub fn new(states: &[String]) -> Self {
        Self {
            states: SelectableList::new(states.to_vec(), None),
            should_be_closed: false,
        }
    }

    pub async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title("States")
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR)
            .style(Style::new().bg(style::ACTIVE_BLOCK_BG));
        Widget::render(&b, area, buf);

        self.states
            .render("", |s| ListItem::from(s.as_str()), b.inner(area), buf);
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_be_closed = true;
            }
            _ => {}
        }
    }

    pub fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }
}
