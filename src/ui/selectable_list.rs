use ratatui::widgets::ListState;

pub struct SelectableList<T> {
    pub items: Vec<T>,
    pub state: ListState,
}

impl<T> SelectableList<T> {
    pub fn new(v: Vec<T>, selected: Option<usize>) -> Self {
        Self {
            items: v,
            state: ListState::default().with_selected(selected),
        }
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
