// SPDX-License-Identifier: MIT

use super::dialog::DialogTrait;
use super::keyboard_handler::KeyboardHandler;
use super::style;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use regex::Regex;

pub struct Dialog {
    title: String,
    text: String,
    input_re: Regex,
    should_be_closed: bool,
}

impl Dialog {
    pub fn new(title: &str, input_re: Regex) -> Self {
        Self {
            title: title.to_string(),
            text: String::new(),
            input_re,
            should_be_closed: false,
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title(self.title.clone())
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        Paragraph::new(self.text.clone() + "_").block(b).render(area, buf);
    }

    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn size(&self) -> Size {
        Size::new(30, 3)
    }
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.should_be_closed = true;
                self.text.clear();
            }
            KeyCode::Char(ch) => {
                if self.input_re.is_match(format!("{}{ch}", self.text).as_str()) {
                    self.text.push(ch);
                }
            }
            KeyCode::Backspace => {
                self.text.pop();
            }
            KeyCode::Enter => {
                self.should_be_closed = true;
            }
            _ => {
                return false;
            }
        }
        true
    }
}
