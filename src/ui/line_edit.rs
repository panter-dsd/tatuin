use super::{
    draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style, widget::WidgetTrait,
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    widgets::{Block, Borders, Paragraph, Widget},
};
use regex::Regex;

pub struct LineEdit {
    text: String,
    validator: Regex,
    last_cursor_pos: Position,
    draw_helper: Option<DrawHelper>,
}

impl LineEdit {
    pub fn new(validator: Regex) -> Self {
        Self {
            text: String::new(),
            validator,
            draw_helper: None,
            last_cursor_pos: Position::default(),
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }
}

#[async_trait]
impl WidgetTrait for LineEdit {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default().borders(Borders::ALL).border_style(style::BORDER_COLOR);
        Paragraph::new(self.text.clone()).block(b).render(area, buf);

        if let Some(dh) = &self.draw_helper {
            let pos = Position::new(
                std::cmp::min(area.x + self.text.len() as u16 + 1, area.x + area.width - 2),
                area.y + 1,
            );

            if pos != self.last_cursor_pos {
                dh.write().await.set_cursor_pos(pos);
                self.last_cursor_pos = pos;
            }
        }
    }

    fn size(&self) -> Size {
        Size::new(30, 3)
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.draw_helper = Some(dh)
    }
}

#[async_trait]
impl KeyboardHandler for LineEdit {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(ch) => {
                if self.validator.is_match(format!("{}{ch}", self.text).as_str()) {
                    self.text.push(ch);
                }
            }
            KeyCode::Backspace => {
                self.text.pop();
            }
            _ => {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl MouseHandler for LineEdit {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
