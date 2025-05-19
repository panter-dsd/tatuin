use super::selectable_list::SelectableList;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

pub struct StatesWidget {
    states: SelectableList<String>,
}

impl StatesWidget {
    pub fn new(states: &[String]) -> Self {
        Self {
            states: SelectableList::new(states.to_vec(), None),
        }
    }

    pub async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.states.render("States", |s| ListItem::from(s.as_str()), area, buf);
    }
}
