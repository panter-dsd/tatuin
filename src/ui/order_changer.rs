use super::WidgetTrait;

pub struct OrderChanger<'a> {
    widgets: Vec<&'a mut dyn WidgetTrait>,
}

fn search_predicate(w: &&mut dyn WidgetTrait) -> bool {
    w.is_enabled() && w.is_visible()
}

impl<'a> OrderChanger<'a> {
    pub fn new(widgets: Vec<&'a mut dyn WidgetTrait>) -> Self {
        let mut s = Self { widgets };
        s.make_all_except_one_inactive();
        s
    }

    fn make_all_except_one_inactive(&mut self) {
        let active_index = self.active_widet_index().unwrap_or(0);
        self.widgets.iter_mut().for_each(|w| w.set_active(false));
        self.widgets[active_index].set_active(true);
    }

    fn active_widet_index(&self) -> Option<usize> {
        self.widgets.iter().position(|w| w.is_active())
    }

    pub fn select_next(&mut self) {
        let next_idx = match self.active_widet_index() {
            Some(idx) => {
                self.widgets[idx].set_active(false);
                println!("HERE {idx}");
                match self.widgets.iter().skip(idx + 1).position(search_predicate) {
                    Some(i) => {
                        println!("HERE_i {i}");
                        idx + i + 1
                    }
                    None => self.widgets.iter().position(search_predicate).unwrap_or(idx),
                }
            }
            None => 0,
        };

        self.widgets[next_idx].set_active(true);
    }

    pub fn select_prev(&mut self) {
        let prev_idx = match self.active_widet_index() {
            Some(idx) => {
                println!(
                    "HERE_PREV {idx}, match={:?}, none={:?}",
                    self.widgets.iter().take(idx).rev().position(search_predicate),
                    self.widgets.iter().rev().position(search_predicate)
                );
                self.widgets[idx].set_active(false);
                match self.widgets.iter().take(idx).rev().position(search_predicate) {
                    Some(i) => idx - i - 1,
                    None => {
                        self.widgets.len() - self.widgets.iter().rev().position(search_predicate).unwrap_or(idx) - 1
                    }
                }
            }
            None => 0,
        };

        println!("PREV_INDEX {prev_idx}");
        self.widgets[prev_idx].set_active(true);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ui::widgets::{Button, WidgetStateTrait};

    #[test]
    fn single_widget_test() {
        let mut button = Button::new("");
        OrderChanger::new(vec![&mut button]).select_next();
        assert!(button.is_active());
        OrderChanger::new(vec![&mut button]).select_prev();
        assert!(button.is_active());
    }

    #[test]
    fn three_widgets_circle_next_test() {
        let mut button1 = Button::new("");
        let mut button2 = Button::new("");
        let mut button3 = Button::new("");
        OrderChanger::new(vec![&mut button1, &mut button2, &mut button3]).select_next();
        assert!(!button1.is_active());
        assert!(button2.is_active());
        assert!(!button3.is_active());
        OrderChanger::new(vec![&mut button1, &mut button2, &mut button3]).select_next();
        assert!(!button1.is_active());
        assert!(!button2.is_active());
        assert!(button3.is_active());
        OrderChanger::new(vec![&mut button1, &mut button2, &mut button3]).select_next();
        assert!(button1.is_active());
        assert!(!button2.is_active());
        assert!(!button3.is_active());
    }

    #[test]
    fn three_widgets_circle_prev_test() {
        let mut button1 = Button::new("");
        let mut button2 = Button::new("");
        let mut button3 = Button::new("");
        OrderChanger::new(vec![&mut button1, &mut button2, &mut button3]).select_prev();
        assert!(!button1.is_active());
        assert!(!button2.is_active());
        assert!(button3.is_active());
        OrderChanger::new(vec![&mut button1, &mut button2, &mut button3]).select_prev();
        assert!(!button1.is_active());
        assert!(button2.is_active());
        assert!(!button3.is_active());
        OrderChanger::new(vec![&mut button1, &mut button2, &mut button3]).select_prev();
        assert!(button1.is_active());
        assert!(!button2.is_active());
        assert!(!button3.is_active());
    }
}
