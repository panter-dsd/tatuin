use std::any::Any;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect, Size},
    text::Text,
    widgets::{Block, Borders, Widget},
};

use crate::{
    provider::Provider,
    types::ArcRwLock,
    ui::{
        draw_helper::DrawHelper,
        keyboard_handler::KeyboardHandler,
        mouse_handler::MouseHandler,
        style,
        tasks_widget::ProvidersStorage,
        widgets::{SpinBox, SpinBoxItem, WidgetTrait},
    },
};

use super::DialogTrait;

const FOOTER: &str = "Input text and press Enter for applying or Esc for cancelling";

pub struct Dialog {
    title: String,
    should_be_closed: bool,
    draw_helper: Option<DrawHelper>,
    providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>,

    provider_selector: SpinBox,
}

impl Dialog {
    pub async fn new(title: &str, providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>) -> Self {
        let provider_items = providers_storage
            .read()
            .await
            .iter()
            .map(|p| SpinBoxItem {
                id: p.name.clone(),
                text: p.name.clone(),
            })
            .collect::<Vec<SpinBoxItem>>();
        let mut provider_selector = SpinBox::new("Provider", &provider_items);
        provider_selector.set_active(true);

        Self {
            title: title.to_string(),
            should_be_closed: false,
            draw_helper: None,
            providers_storage,
            provider_selector,
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

        let [provider_area, _] = Layout::vertical([
            Constraint::Length(self.provider_selector.size().height),
            Constraint::Fill(1),
        ])
        .areas(inner_area);
        self.provider_selector.render(provider_area, buf).await;
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
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
            self.should_be_closed = true;
            return true;
        }

        if self.provider_selector.is_active() {
            return self.provider_selector.handle_key(key).await;
        }

        true
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
