// SPDX-License-Identifier: MIT

use std::any::Any;

use crate::ui::{draw_helper::DrawHelper, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler};

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::Style,
};

use async_trait::async_trait;

pub trait WidgetStateTrait {
    fn is_active(&self) -> bool;
    fn set_active(&mut self, is_active: bool);
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, is_enabled: bool);
}

pub struct WidgetState {
    is_active: bool,
    is_enabled: bool,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            is_active: false,
            is_enabled: true,
        }
    }
}

impl WidgetStateTrait for WidgetState {
    fn is_active(&self) -> bool {
        self.is_active
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    fn set_enabled(&mut self, is_enabled: bool) {
        self.is_enabled = is_enabled;
    }
}

#[macro_export]
macro_rules! impl_state_trait {
    ($struct_name:ident) => {
        impl WidgetStateTrait for $struct_name {
            fn is_active(&self) -> bool {
                self.widget_state.is_active()
            }

            fn set_active(&mut self, is_active: bool) {
                self.widget_state.set_active(is_active);
            }

            fn is_enabled(&self) -> bool {
                self.widget_state.is_enabled()
            }

            fn set_enabled(&mut self, is_enabled: bool) {
                self.widget_state.set_enabled(is_enabled);
            }
        }
    };
}

#[async_trait]
pub trait WidgetTrait: WidgetStateTrait + KeyboardHandler + MouseHandler + Send + Sync {
    async fn render(&mut self, area: Rect, buf: &mut Buffer);
    fn size(&self) -> Size;
    fn set_draw_helper(&mut self, _dh: DrawHelper) {}
    fn set_pos(&mut self, _pos: Position) {}
    fn style(&self) -> Style {
        Style::default()
    }
    fn set_style(&mut self, _style: Style) {}
    fn as_any(&self) -> &dyn Any;
}
