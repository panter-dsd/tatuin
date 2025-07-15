use super::WidgetTrait;

pub struct OrderChanger<'a> {
    widgets: Vec<&'a mut dyn WidgetTrait>,
}

impl<'a> OrderChanger<'a> {
    pub fn new(widgets: Vec<&'a mut dyn WidgetTrait>) -> Self {
        Self { widgets }
    }

    fn active_widet_index(&self) -> Option<usize> {
        self.widgets.iter().position(|w| w.is_active())
    }

    pub fn select_next(&mut self) {
        let next_idx = match self.active_widet_index() {
            Some(idx) => {
                self.widgets[idx].set_active(false);
                if idx == self.widgets.len() - 1 { 0 } else { idx + 1 }
            }
            None => 0,
        };

        self.widgets[next_idx].set_active(true);
    }

    pub fn select_prev(&mut self) {
        let prev_idx = match self.active_widet_index() {
            Some(idx) => {
                self.widgets[idx].set_active(false);
                if idx == 0 { self.widgets.len() - 1 } else { idx - 1 }
            }
            None => 0,
        };

        self.widgets[prev_idx].set_active(true);
    }
}
