use std::any::Any;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect, Size},
    text::Text,
    widgets::{Block, Borders, Widget},
};

use crate::ui::{
    draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style,
    widgets::WidgetTrait,
};

use super::DialogTrait;

const FOOTER: &str = "Input text and press Enter for applying or Esc for cancelling";

pub struct Dialog {
    title: String,
    should_be_closed: bool,
    draw_helper: Option<DrawHelper>,
}

impl Dialog {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            should_be_closed: false,
            draw_helper: None,
        }
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl WidgetTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let b = Block::default()
            .title_top(self.title.clone())
            .title_bottom(FOOTER)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR);
        let inner_area = b.inner(area);
        b.render(area, buf);
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.draw_helper = Some(dh);
    }

    fn size(&self) -> Size {
        Size::new(Text::from(FOOTER).width() as u16 + 2, 20)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.should_be_closed = true;
            }
            KeyCode::Enter => {
                self.should_be_closed = true;
            }
            _ => {
                return false;
            }
        }

        if self.should_be_closed && self.draw_helper.is_some() {
            self.draw_helper.as_ref().unwrap().write().await.hide_cursor();
        }

        true
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
