// SPDX-License-Identifier: MIT

use crate::state::StateSettings;
use crate::task::DateTimeUtc;
use chrono::Local;
use ratatui::text::Text;

use super::dialog::DialogTrait;
use super::mouse_handler::MouseHandler;
use super::selectable_list::SelectableList;
use super::{AppBlockWidget, style};
use crate::task::datetime_to_str;
use async_trait::async_trait;
use crossterm::event::MouseEvent;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::widgets::{Block, Borders, ListItem, Widget};

pub struct Dialog {
    title: String,
    title_width: u16,
    items: SelectableList<String>,
    should_be_closed: bool,
    selected_item: Option<String>,
}

impl Dialog {
    pub fn new(due: Option<DateTimeUtc>) -> Self {
        let title = format!("Current due {}", datetime_to_str(due, &Local::now().timezone()));
        let title_width = Text::from(title.as_str()).width() as u16;
        Self {
            title,
            title_width,
            items: SelectableList::new(
                vec![
                    String::from("Today"),
                    String::from("Tomorrow"),
                    String::from("This weekend"),
                    String::from("Next week(Monday)"),
                    String::from("No date"),
                ],
                Some(0),
            ),
            should_be_closed: false,
            selected_item: None,
        }
    }

    pub fn selected_state(&self) -> &Option<String> {
        &self.selected_item
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top(self.title.as_str())
            .title_bottom("Use j/k (up/down) for moving and Enter for applying")
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        Widget::render(&b, area, buf);

        self.items
            .render("", |s| ListItem::from(s.as_str()), b.inner(area), buf);
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_be_closed = true;
            }
            KeyCode::Char('j') | KeyCode::Down => self.items.select_next().await,
            KeyCode::Char('k') | KeyCode::Up => self.items.select_previous().await,
            KeyCode::Char('g') | KeyCode::Home => self.items.select_first().await,
            KeyCode::Char('G') | KeyCode::End => self.items.select_last().await,
            KeyCode::Enter => {
                self.should_be_closed = true;
                if let Some(s) = self.items.selected() {
                    self.selected_item = Some(s.clone());
                }
            }
            _ => {}
        }
    }

    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn size(&self) -> Size {
        let mut s = self.items.size();
        s.height += 2;
        s.width = std::cmp::max(s.width, self.title_width) + 2;
        s
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
