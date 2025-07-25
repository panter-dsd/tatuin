// SPDX-License-Identifier: MIT

use std::any::Any;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect, Size},
    style::Style,
    text::Text,
    widgets::Widget,
};

use super::WidgetTrait;
use crate::{
    task::DateTimeUtc,
    time::clear_time,
    ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style},
};

#[derive(PartialEq, Eq)]
enum Element {
    Year,
    Month,
    Day,
}

pub struct DateTimeEditor {
    dt: DateTimeUtc,
    current_element: Element,
    is_active: bool,
}

impl DateTimeEditor {
    pub fn new(dt: Option<DateTimeUtc>) -> Self {
        Self {
            dt: clear_time(&dt.unwrap_or(chrono::Local::now().to_utc())),
            current_element: Element::Day,
            is_active: false,
        }
    }

    pub fn value(&self) -> DateTimeUtc {
        self.dt
    }

    fn style(&self, element: Element) -> Style {
        if self.is_active && self.current_element == element {
            style::DATE_TIME_EDITOR_ACTIVE_ELEMENT
        } else {
            style::DATE_TIME_EDITOR_INACTIVE_ELEMENT
        }
    }

    fn suffix(&self, element: Element) -> &str {
        if self.is_active && self.current_element == element {
            return "↕";
        }

        if element == Element::Day { " " } else { "-" }
    }
}

#[async_trait]
impl WidgetTrait for DateTimeEditor {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [
            year_area,
            year_suffix_area,
            month_area,
            month_suffix_area,
            day_area,
            day_suffix_area,
        ] = Layout::horizontal([
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .areas(area);

        let suffix_style = style::DATE_TIME_EDITOR_INACTIVE_ELEMENT;
        Text::styled(format!("{}", self.dt.format("%Y")), self.style(Element::Year)).render(year_area, buf);
        Text::styled(self.suffix(Element::Year), suffix_style).render(year_suffix_area, buf);

        Text::styled(format!("{}", self.dt.format("%m")), self.style(Element::Month)).render(month_area, buf);
        Text::styled(self.suffix(Element::Month), suffix_style).render(month_suffix_area, buf);

        Text::styled(format!("{}", self.dt.format("%d")), self.style(Element::Day)).render(day_area, buf);
        if self.is_active && self.current_element == Element::Day {
            Text::styled(self.suffix(Element::Day), suffix_style).render(day_suffix_area, buf);
        }
    }

    fn size(&self) -> Size {
        Size::new(Text::from("yyyy-mm-dd").width() as u16, 1)
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for DateTimeEditor {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_active {
            return false;
        }

        match key.code {
            KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => match self.current_element {
                Element::Year => {
                    return false;
                }
                Element::Month => self.current_element = Element::Year,
                Element::Day => self.current_element = Element::Month,
            },
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => match self.current_element {
                Element::Year => self.current_element = Element::Month,
                Element::Month => self.current_element = Element::Day,
                Element::Day => {
                    return false;
                }
            },
            KeyCode::Char('k') | KeyCode::Up => {
                self.dt = match self.current_element {
                    Element::Year => self.dt.checked_add_months(chrono::Months::new(12)).unwrap_or(self.dt),
                    Element::Month => self.dt.checked_add_months(chrono::Months::new(1)).unwrap_or(self.dt),
                    Element::Day => self.dt.checked_add_days(chrono::Days::new(1)).unwrap_or(self.dt),
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.dt = match self.current_element {
                    Element::Year => self.dt.checked_sub_months(chrono::Months::new(12)).unwrap_or(self.dt),
                    Element::Month => self.dt.checked_sub_months(chrono::Months::new(1)).unwrap_or(self.dt),
                    Element::Day => self.dt.checked_sub_days(chrono::Days::new(1)).unwrap_or(self.dt),
                }
            }
            _ => {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl MouseHandler for DateTimeEditor {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
