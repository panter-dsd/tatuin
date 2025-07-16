use std::any::Any;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    text::Text,
    widgets::{Block, Borders, Widget},
};
use tokio::sync::broadcast;

use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};

use super::{WidgetState, WidgetStateTrait, WidgetTrait};

pub struct Button {
    title: String,
    width: u16,
    tx: broadcast::Sender<()>,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(Button);

impl Button {
    pub fn new(title: &str) -> Self {
        let width = Text::from(title).width() as u16 + 2;
        let (tx, _) = broadcast::channel(1);
        Self {
            title: title.to_string(),
            width,
            tx,
            widget_state: WidgetState::default(),
        }
    }

    pub fn on_pressed_subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Button title={} is_active={}",
            self.title,
            self.widget_state.is_active()
        )
    }
}

#[async_trait]
impl WidgetTrait for Button {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style::BORDER_COLOR)
            .style(if self.widget_state.is_active() {
                style::ACTIVE_BUTTON_STYLE
            } else {
                style::INACTIVE_BUTTON_STYLE
            });
        let inner_area = block.inner(area);
        block.render(area, buf);

        Text::from(self.title.as_str()).render(inner_area, buf);
    }

    fn size(&self) -> Size {
        Size::new(self.width, 3)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Button {
    #[tracing::instrument(level = "debug")]
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.widget_state.is_active() {
            return false;
        }

        match key.code {
            KeyCode::Enter => {
                let _ = self.tx.send(());
            }
            _ => {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl MouseHandler for Button {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
