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
        order_changer::OrderChanger,
        style,
        tasks_widget::ProvidersStorage,
        widgets::{ComboBox, ComboBoxItem, WidgetTrait},
    },
};

use super::DialogTrait;

const FOOTER: &str = "Input text and press Enter for applying or Esc for cancelling";

pub struct Dialog {
    title: String,
    should_be_closed: bool,
    draw_helper: Option<DrawHelper>,
    providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>,

    provider_selector: ComboBox,
    project_selector: ComboBox,
}

impl Dialog {
    pub async fn new(title: &str, providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>) -> Self {
        let provider_items = providers_storage
            .read()
            .await
            .iter()
            .map(|p| ComboBoxItem {
                text: p.name.clone(),
                data: String::new(),
            })
            .collect::<Vec<ComboBoxItem>>();
        let mut provider_selector = ComboBox::new("Provider", &provider_items);
        provider_selector.set_active(true);

        Self {
            title: title.to_string(),
            should_be_closed: false,
            draw_helper: None,
            providers_storage,
            provider_selector,
            project_selector: ComboBox::new("Project", &[]),
        }
    }

    fn order_calculator(&mut self) -> OrderChanger<'_> {
        OrderChanger::new(vec![&mut self.provider_selector, &mut self.project_selector])
    }

    fn next_widget(&mut self) {
        self.order_calculator().select_next();
    }

    fn prev_widget(&mut self) {
        self.order_calculator().select_prev();
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

        let [provider_and_project_area, _] = Layout::vertical([
            Constraint::Length(self.provider_selector.size().height),
            Constraint::Fill(1),
        ])
        .areas(inner_area);

        let [provider_area, project_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(provider_and_project_area);

        let mut to_render = vec![
            (&mut self.provider_selector, provider_area),
            (&mut self.project_selector, project_area),
        ];

        // the active should render last
        to_render.sort_by(|l, r| {
            if l.0.is_active() {
                std::cmp::Ordering::Greater
            } else if r.0.is_active() {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });
        for (w, a) in to_render {
            w.render(a, buf).await;
        }
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
        if self.provider_selector.is_active() {
            let handled = self.provider_selector.handle_key(key).await;
            if let Some(item) = self.provider_selector.value().await {
                let mut providers = self.providers_storage.write().await;
                let provider = providers.iter_mut().find(|p| p.name == item.text);
                if let Some(p) = provider.as_ref() {
                    if let Ok(projects) = p.provider.write().await.projects().await {
                        tracing::debug!(target:"add_edit_task", projects=?projects);
                        self.project_selector
                            .set_items(
                                &projects
                                    .iter()
                                    .map(|p| ComboBoxItem {
                                        text: p.name(),
                                        data: p.id(),
                                    })
                                    .collect::<Vec<ComboBoxItem>>(),
                            )
                            .await;
                    }
                }
            }

            if handled {
                return true;
            }
        }

        if self.project_selector.is_active() && self.project_selector.handle_key(key).await {
            return true;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.should_be_closed = true;
            }
            KeyCode::Tab => {
                self.next_widget();
            }
            KeyCode::BackTab => {
                self.prev_widget();
            }
            _ => return false,
        }
        true
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
