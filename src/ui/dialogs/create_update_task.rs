// SPDX-License-Identifier: MIT

use std::{any::Any, sync::Arc};

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect, Size},
    widgets::{Block, Borders, Widget},
};
use tatuin_core::{
    task::{DateTimeUtc, Priority, Task as TaskTrait},
    task_patch::{DuePatchItem, TaskPatch, ValuePatch},
    types::ArcRwLock,
};

use crate::ui::{
    draw_helper::DrawHelper,
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    order_changer::OrderChanger,
    style,
    tasks_widget::ProvidersStorage,
    widgets::{
        Button, ComboBox, ComboBoxItem, CustomWidgetItemUpdater, DateEditor, LineEdit, Text, TextEdit, WidgetState,
        WidgetStateTrait, WidgetTrait,
    },
};

use super::DialogTrait;

const CUSTOM_DUE_TEXT: &str = "Custom";

struct ComboBoxItemUpdater {}

impl CustomWidgetItemUpdater<DuePatchItem> for ComboBoxItemUpdater {
    fn update(&self, w: Arc<dyn WidgetTrait>, item: &mut ComboBoxItem<DuePatchItem>) {
        let editor = w.as_any().downcast_ref::<DateEditor>().unwrap();
        let due = DuePatchItem::Custom(editor.value());
        item.set_data(&due);
        item.set_display_text(&due.to_string());
    }
}

pub struct Dialog {
    title: String,
    should_be_closed: bool,
    add_another_one: bool,
    draw_helper: Option<DrawHelper>,
    providers_storage: ArcRwLock<dyn ProvidersStorage>,
    widget_state: WidgetState,
    size: Size,
    task: Option<Box<dyn TaskTrait>>,

    provider_selector: ComboBox<String>,
    project_selector: ComboBox<String>,

    task_name_caption: Text,
    task_name_editor: LineEdit,

    task_description_caption: Text,
    task_description_editor: TextEdit,

    priority_selector: ComboBox<Priority>,
    due_date_selector: ComboBox<DuePatchItem>,

    create_task_button: Button,
    create_task_and_another_one: Button,
}
crate::impl_widget_state_trait!(Dialog);

impl Dialog {
    pub async fn new(title: &str, providers_storage: ArcRwLock<dyn ProvidersStorage>) -> Self {
        let provider_items = providers_storage
            .read()
            .await
            .iter()
            .filter(|p| p.capabilities.create_task)
            .map(|p| p.name.clone().into())
            .collect::<Vec<ComboBoxItem<_>>>();

        let mut due_date_selector = ComboBox::new(
            "Due date",
            &DuePatchItem::values()
                .iter()
                .map(|d| ComboBoxItem::new(d.to_string().as_str(), *d))
                .collect::<Vec<ComboBoxItem<DuePatchItem>>>(),
        )
        .current_item(&ComboBoxItem::new(
            DuePatchItem::Today.to_string().as_str(),
            DuePatchItem::Today,
        ))
        .await;

        due_date_selector
            .add_custom_widget(
                ComboBoxItem::new(CUSTOM_DUE_TEXT, DuePatchItem::Custom(DateTimeUtc::default())),
                Arc::new(DateEditor::new(None)),
                Arc::new(ComboBoxItemUpdater {}),
            )
            .await;

        let mut s = Self {
            title: title.to_string(),
            should_be_closed: false,
            add_another_one: false,
            draw_helper: None,
            providers_storage,
            provider_selector: ComboBox::new("Provider", &provider_items),
            widget_state: WidgetState::default(),
            size: Size::new(100, 20),
            task: None,
            project_selector: ComboBox::new("Project", &[]),
            task_name_caption: Text::new("Task name"),
            task_name_editor: LineEdit::new(None),
            task_description_caption: Text::new("Task description"),
            task_description_editor: TextEdit::new(),
            priority_selector: ComboBox::new(
                "Priority",
                &Priority::values()
                    .iter()
                    .map(|p| ComboBoxItem::new(p.to_string().as_str(), *p))
                    .collect::<Vec<ComboBoxItem<Priority>>>(),
            )
            .current_item(&ComboBoxItem::new(
                Priority::default().to_string().as_str(),
                Priority::default(),
            ))
            .await,
            due_date_selector,
            create_task_button: Button::new("Create a task and close\nCtrl+Enter"),
            create_task_and_another_one: Button::new("Create a task\nShift+Enter"),
        };
        s.provider_selector.set_active(true);
        s.update_enabled_state().await;
        s
    }

    fn is_task_creation(&self) -> bool {
        self.task.is_none()
    }

    pub async fn set_task(&mut self, task: &dyn TaskTrait) {
        self.task = Some(task.clone_boxed());
        self.create_task_and_another_one.set_visible(false);
        self.create_task_button
            .set_title("Update the task and close\nCtrl+Enter");
        self.provider_selector.set_current_item(&task.provider().into()).await;
        self.fill_project_selector_items().await;
        self.fill_priority_selector_items().await;
        if let Some(p) = task.project() {
            let item = ComboBoxItem::new(p.name().as_str(), p.id());
            let found = self.project_selector.set_current_item(&item).await;
            if !found {
                self.project_selector.add_item(item.clone()).await;
                self.project_selector.set_current_item(&item).await;
            }
        } else {
            self.project_selector.set_current_item_index(&Some(0)).await;
        }
        self.task_name_editor.set_text(task.text().as_str());
        if let Some(d) = task.description() {
            self.task_description_editor.set_text(&d);
        }
        self.priority_selector
            .set_current_item(&ComboBoxItem::new(
                task.priority().to_string().as_str(),
                task.priority(),
            ))
            .await;

        let task_due = task.due();
        let due: DuePatchItem = task_due.map_or(DuePatchItem::NoDate, |d| d.into());
        if let Some(dt) = task_due {
            self.due_date_selector.remove_all_custom_widgets().await;
            let custom_due = DuePatchItem::Custom(dt);
            self.due_date_selector
                .add_custom_widget(
                    ComboBoxItem::new(CUSTOM_DUE_TEXT, custom_due).display(custom_due.to_string().as_str()),
                    Arc::new(DateEditor::new(Some(dt))),
                    Arc::new(ComboBoxItemUpdater {}),
                )
                .await;
        }
        self.due_date_selector
            .set_current_item(&ComboBoxItem::new(
                match due {
                    DuePatchItem::Custom(_) => CUSTOM_DUE_TEXT.to_string(),
                    _ => due.to_string(),
                }
                .as_str(),
                due,
            ))
            .await;

        self.provider_selector.set_active(false);
        self.task_name_editor.set_active(true);
        self.update_enabled_state().await
    }

    pub async fn provider_name(&self) -> Option<String> {
        if !self.can_create_task() {
            return None;
        }

        self.provider_selector.value().await.map(|item| item.text().to_string())
    }

    pub async fn project_id(&self) -> Option<String> {
        if !self.can_create_task() {
            return None;
        }

        self.project_selector.value().await.map(|item| item.data().to_string())
    }

    pub async fn task_patch(&self) -> Option<TaskPatch> {
        if !self.can_create_task() {
            return None;
        }

        let description = self.task_description_editor.text();

        Some(TaskPatch {
            task: self.task.as_ref().map(|t| t.clone_boxed()),
            name: ValuePatch::Value(self.task_name_editor.text()),
            description: if description.is_empty() {
                ValuePatch::NotSet
            } else {
                ValuePatch::Value(description)
            },
            due: self.due_date_selector.value().await.map(|item| *item.data()).into(),
            priority: self.priority_selector.value().await.map(|item| *item.data()).into(),
            state: ValuePatch::NotSet,
        })
    }

    pub fn add_another_one(&self) -> bool {
        self.add_another_one
    }

    fn order_calculator(&mut self) -> OrderChanger<'_> {
        OrderChanger::new(vec![
            &mut self.provider_selector,
            &mut self.project_selector,
            &mut self.task_name_editor,
            &mut self.task_description_editor,
            &mut self.priority_selector,
            &mut self.due_date_selector,
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

    fn can_create_task(&self) -> bool {
        self.task_name_editor.is_enabled() && !self.task_name_editor.text().is_empty()
    }

    async fn update_enabled_state(&mut self) {
        let provider_selected = self.provider_selector.value().await.is_some();
        let project_selected = self.project_selector.value().await.is_some();
        self.provider_selector.set_enabled(self.is_task_creation());
        self.project_selector
            .set_enabled(self.is_task_creation() && provider_selected);

        self.task_name_editor.set_enabled(provider_selected && project_selected);

        let can_create_task = self.can_create_task();
        self.task_description_editor.set_enabled(can_create_task);
        self.priority_selector.set_enabled(self.task_name_editor.is_enabled());
        self.due_date_selector.set_enabled(self.task_name_editor.is_enabled());

        self.create_task_button.set_enabled(can_create_task);
        self.create_task_and_another_one.set_enabled(can_create_task);
        self.create_task_button.set_active(self.create_task_button.is_enabled());
        self.create_task_and_another_one
            .set_active(self.create_task_and_another_one.is_enabled());
    }

    async fn fill_project_selector_items(&mut self) {
        let provider_name = self.provider_selector.value().await.map(|item| item.text().to_string());
        if provider_name.is_none() {
            return;
        }
        let provider_name = provider_name.unwrap();

        let mut providers = self.providers_storage.write().await;
        let provider = providers.iter_mut().find(|p| p.name == provider_name);
        if provider.is_none() {
            return;
        }
        let provider = provider.unwrap();
        if let Ok(projects) = provider.provider.write().await.projects().await {
            self.project_selector
                .set_items(
                    &projects
                        .iter()
                        .map(|p| ComboBoxItem::new(p.name().as_str(), p.id()))
                        .collect::<Vec<ComboBoxItem<_>>>(),
                )
                .await;
        }
    }

    async fn fill_priority_selector_items(&mut self) {
        let provider_name = self.provider_selector.value().await.map(|item| item.text().to_string());
        if provider_name.is_none() {
            return;
        }
        let provider_name = provider_name.unwrap();

        let mut providers = self.providers_storage.write().await;
        let provider = providers.iter_mut().find(|p| p.name == provider_name);
        if provider.is_none() {
            return;
        }

        let provider = provider.unwrap();
        self.priority_selector
            .set_items(
                &provider
                    .supported_priorities
                    .iter()
                    .map(|p| ComboBoxItem::new(p.to_string().as_str(), *p))
                    .collect::<Vec<ComboBoxItem<_>>>(),
            )
            .await;
        self.priority_selector
            .set_current_item(&ComboBoxItem::new(
                Priority::default().to_string().as_str(),
                Priority::default(),
            ))
            .await;
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
        let b = Block::new()
            .style(style::default_style())
            .title_top(self.title.clone())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(style::border_color());
        let inner_area = b.inner(area);
        b.render(area, buf);

        self.task_name_caption.set_size(inner_area.as_size());
        self.task_description_caption.set_size(inner_area.as_size());
        self.task_description_editor.set_size(Size::new(inner_area.width, 5));

        let [
            provider_and_project_area,
            task_name_caption_area,
            task_name_editor_area,
            task_description_caption_area,
            task_description_editor_area,
            priority_and_due_area,
            _,
            buttons_area,
        ] = Layout::vertical([
            Constraint::Length(self.provider_selector.size().height),
            Constraint::Length(self.task_name_caption.size().height),
            Constraint::Length(self.task_name_editor.size().height),
            Constraint::Length(self.task_description_caption.size().height),
            Constraint::Length(self.task_description_editor.size().height),
            Constraint::Length(self.priority_selector.size().height),
            Constraint::Fill(1),
            Constraint::Length(self.create_task_button.size().height),
        ])
        .areas(inner_area);

        let [provider_area, _, project_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1)])
                .areas(provider_and_project_area);

        let [priority_area, _, due_date_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1)])
                .areas(priority_and_due_area);

        let max_buttons_width = self
            .create_task_button
            .size()
            .width
            .max(self.create_task_and_another_one.size().width);
        let [
            _,
            create_task_button_area,
            _,
            create_task_and_another_one_button_area,
            _,
        ] = if self.is_task_creation() {
            Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Length(max_buttons_width),
                Constraint::Length(5),
                Constraint::Length(max_buttons_width),
                Constraint::Fill(1),
            ])
            .areas(buttons_area)
        } else {
            Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(0),
                Constraint::Fill(0),
                Constraint::Fill(1),
            ])
            .areas(buttons_area)
        };

        let mut to_render: Vec<(&mut dyn WidgetTrait, Rect)> = vec![
            (&mut self.create_task_button, create_task_button_area),
            (
                &mut self.create_task_and_another_one,
                create_task_and_another_one_button_area,
            ),
            (&mut self.provider_selector, provider_area),
            (&mut self.project_selector, project_area),
            (&mut self.task_name_caption, task_name_caption_area),
            (&mut self.task_name_editor, task_name_editor_area),
            (&mut self.task_description_caption, task_description_caption_area),
            (&mut self.task_description_editor, task_description_editor_area),
            (&mut self.priority_selector, priority_area),
            (&mut self.due_date_selector, due_date_area),
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
            if w.is_visible() {
                w.render(a, buf).await;
            }
        }
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.provider_selector.set_draw_helper(dh.clone());
        self.project_selector.set_draw_helper(dh.clone());
        self.task_name_editor.set_draw_helper(dh.clone());
        self.task_description_editor.set_draw_helper(dh.clone());
        self.draw_helper = Some(dh);
    }

    fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    fn min_size(&self) -> Size {
        Size::new(90, 23)
    }

    fn size(&self) -> Size {
        self.size
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.can_create_task() && key.code == KeyCode::Enter {
            let mut handled = true;
            match key.modifiers {
                KeyModifiers::CONTROL => self.should_be_closed = true,
                KeyModifiers::SHIFT => {
                    self.should_be_closed = true;
                    self.add_another_one = true;
                }
                _ => handled = false,
            }
            if handled {
                return true;
            }
        }

        let current_provider = self.provider_selector.value().await;
        if self.provider_selector.is_active() && self.provider_selector.handle_key(key).await {
            let new_provider = self.provider_selector.value().await;
            if current_provider != new_provider {
                self.fill_project_selector_items().await;
                self.fill_priority_selector_items().await;
            }
            self.update_enabled_state().await;
            return true;
        }

        if self.project_selector.is_active() && self.project_selector.handle_key(key).await {
            self.update_enabled_state().await;
            return true;
        }

        if self.task_name_editor.is_active() && key.code == KeyCode::Enter {
            // Move to description on Enter
            self.next_widget().await;
            return true;
        }

        if self.task_name_editor.is_active() && self.task_name_editor.handle_key(key).await {
            self.update_enabled_state().await;
            return true;
        }

        if self.task_description_editor.is_active() && self.task_description_editor.handle_key(key).await {
            self.update_enabled_state().await;
            return true;
        }

        if self.priority_selector.is_active() && self.priority_selector.handle_key(key).await {
            return true;
        }

        if self.due_date_selector.is_active() && self.due_date_selector.handle_key(key).await {
            return true;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.should_be_closed = true;
                self.task_name_editor.clear(); // to make can_create_task return false
            }
            KeyCode::Tab => {
                self.next_widget().await;
            }
            KeyCode::BackTab => {
                self.prev_widget().await;
            }
            _ => {}
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
