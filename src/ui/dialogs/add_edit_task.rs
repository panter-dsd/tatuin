use std::any::Any;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect, Size},
    text::Text as RatatuiText,
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
        widgets::{ComboBox, ComboBoxItem, LineEdit, Text, WidgetState, WidgetStateTrait, WidgetTrait},
    },
};

use super::DialogTrait;

const FOOTER: &str = "Input text and press Enter for applying or Esc for cancelling";

pub struct Dialog {
    title: String,
    should_be_closed: bool,
    draw_helper: Option<DrawHelper>,
    providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>,
    widget_state: WidgetState,

    provider_selector: ComboBox,
    project_selector: ComboBox,

    task_name_caption: Text,
    task_name_editor: LineEdit,
}
crate::impl_widget_state_trait!(Dialog);

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
            widget_state: WidgetState::default(),
            project_selector: ComboBox::new("Project", &[]),
            task_name_caption: Text::new("Task name"),
            task_name_editor: LineEdit::new(None),
        }
    }

    fn order_calculator(&mut self) -> OrderChanger<'_> {
        OrderChanger::new(vec![
            &mut self.provider_selector,
            &mut self.project_selector,
            &mut self.task_name_editor,
        ])
    }

    async fn next_widget(&mut self) {
        self.order_calculator().select_next();
        self.hide_cursor().await;
    }

    async fn prev_widget(&mut self) {
        self.order_calculator().select_prev();
        self.hide_cursor().await;
    }

    async fn hide_cursor(&mut self) {
        if let Some(dh) = &self.draw_helper {
            dh.write().await.hide_cursor();
        }
    }

    async fn update_enabled_state(&mut self) {
        let provider_selected = self.provider_selector.value().await.is_some();
        let project_selected = self.project_selector.value().await.is_some();
        self.project_selector.set_enabled(provider_selected);
        self.task_name_editor.set_enabled(provider_selected && project_selected);
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

        let [provider_and_project_area, task_name_area, _] = Layout::vertical([
            Constraint::Length(self.provider_selector.size().height),
            Constraint::Length(self.task_name_editor.size().height),
            Constraint::Fill(1),
        ])
        .areas(inner_area);

        let [provider_area, project_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(provider_and_project_area);

        let [mut task_name_caption_area, task_name_editor_area] = Layout::horizontal([
            Constraint::Length(self.task_name_caption.size().width),
            Constraint::Fill(1),
        ])
        .areas(task_name_area);
        task_name_caption_area.y += 1;

        let mut to_render: Vec<(&mut dyn WidgetTrait, Rect)> = vec![
            (&mut self.provider_selector, provider_area),
            (&mut self.project_selector, project_area),
            (&mut self.task_name_caption, task_name_caption_area),
            (&mut self.task_name_editor, task_name_editor_area),
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
        self.task_name_editor.set_draw_helper(dh.clone());
        self.draw_helper = Some(dh);
    }

    fn size(&self) -> Size {
        Size::new(RatatuiText::from(FOOTER).width() as u16 + 2, 20)
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

        if self.task_name_editor.is_active() && self.task_name_editor.handle_key(key).await {
            return true;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.should_be_closed = true;
            }
            KeyCode::Tab => {
                self.next_widget().await;
            }
            KeyCode::BackTab => {
                self.prev_widget().await;
            }
            _ => return false,
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
