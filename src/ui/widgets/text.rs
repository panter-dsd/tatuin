// SPDX-License-Identifier: MIT

use std::any::Any;

use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::{Modifier, Style},
    text::Text as RatatuiText,
    widgets::Widget,
};

use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};

use super::{WidgetState, WidgetStateTrait, WidgetTrait};

pub struct Text {
    text: String,
    width: u16,
    style: Style,
    modifier: Modifier,
    widget_state: WidgetState,
}
crate::impl_state_trait!(Text);

impl Text {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            width: RatatuiText::from(text).width() as u16,
            style: style::REGULAR_TEXT_STYLE,
            modifier: Modifier::empty(),
            widget_state: WidgetState::default(),
        }
    }

    pub fn modifier(mut self, m: Modifier) -> Self {
        self.modifier = m;
        self
    }

    pub fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }
}

#[async_trait]
impl WidgetTrait for Text {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        RatatuiText::styled(self.text.as_str(), self.style.add_modifier(self.modifier))
            .centered()
            .render(
                Rect {
                    x: area.x,
                    y: area.y,
                    width: self.width,
                    height: 1,
                },
                buf,
            );
    }

    fn size(&self) -> Size {
        Size::new(self.width, 1)
    }

    fn set_style(&mut self, style: Style) {
        self.style = style
    }

    fn style(&self) -> Style {
        self.style
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Text {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

#[async_trait]
impl MouseHandler for Text {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
