// SPDX-License-Identifier: MIT

use std::{any::Any, sync::Arc};

use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect, Size},
    widgets::{Clear, Widget},
};
use tokio::sync::RwLock;

use super::{Button, LineEdit, Text, WidgetState, WidgetStateTrait, WidgetTrait};
use crate::{
    types::ArcRwLock,
    ui::{
        dialogs::{DialogTrait, ListDialog},
        keyboard_handler::KeyboardHandler,
        mouse_handler::MouseHandler,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Item<T> {
    text: String,
    display: String,
    data: T,
}

impl<T> Item<T>
where
    T: Clone,
{
    pub fn new(text: &str, data: T) -> Self {
        Self {
            text: text.to_string(),
            display: text.to_string(),
            data,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_display_text(&mut self, display: &str) {
        self.display = display.to_string();
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn set_data(&mut self, data: &T) {
        self.data = data.clone();
    }
}

pub trait CustomWidgetItemUpdater<T>: Sync + Send {
    fn update(&self, w: Arc<dyn WidgetTrait>, item: &mut Item<T>);
}

struct InternalData<T> {
    items: Vec<Item<T>>,
    selected: Option<Item<T>>,
    custom_widgets: Vec<Arc<dyn WidgetTrait>>,
    custom_widget_item_updaters: Vec<Arc<dyn CustomWidgetItemUpdater<T>>>,
    dialog: Option<ListDialog<String>>,
}

pub struct ComboBox<T> {
    caption: Text,
    editor: LineEdit,
    button: Button,
    widget_state: WidgetState,
    internal_data: ArcRwLock<InternalData<T>>,
}

impl<T> WidgetStateTrait for ComboBox<T> {
    fn is_active(&self) -> bool {
        self.widget_state.is_active()
    }

    fn set_active(&mut self, is_active: bool) {
        self.widget_state.set_active(is_active);
        self.button.set_active(is_active);
    }

    fn is_enabled(&self) -> bool {
        self.widget_state.is_enabled()
    }

    fn set_enabled(&mut self, is_enabled: bool) {
        self.widget_state.set_enabled(is_enabled);
        self.button.set_enabled(is_enabled);
    }

    fn is_visible(&self) -> bool {
        self.widget_state.is_visible()
    }

    fn set_visible(&mut self, is_visible: bool) {
        self.widget_state.set_visible(is_visible);
    }
}

impl<T> std::fmt::Debug for ComboBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ComboBox")
    }
}

impl<T> ComboBox<T>
where
    T: Clone + Sync + Send + Eq + 'static,
{
    pub fn new(caption: &str, items: &[Item<T>]) -> Self {
        let button = Button::new("▽");

        let internal_data = Arc::new(RwLock::new(InternalData {
            items: items.to_vec(),
            selected: None,
            custom_widgets: Vec::new(),
            custom_widget_item_updaters: Vec::new(),
            dialog: None,
        }));

        tokio::spawn({
            let internal_data = internal_data.clone();
            let mut rx = button.on_pressed_subscribe();

            async move {
                loop {
                    tokio::select! {
                        r = rx.recv() => {
                            if r.is_err() {
                                return;
                            }
                            let mut data = internal_data.write().await;
                            let custom_widgets_count = data.custom_widgets.len();
                            let items = data.items.iter().take(data.items.len() - custom_widgets_count)
                                        .map(|item| item.text.clone()).collect::<Vec<String>>();
                            if !items.is_empty() {
                                let selected = data.selected.as_ref().map(|item| item.text.clone()).unwrap_or_default();
                                let mut d = ListDialog::new(&items, selected.as_str()).show_top_title(false).show_bottom_title(false);

                                // clear active flag
                                for w in data.custom_widgets.iter_mut() {
                                    Arc::get_mut(w).unwrap().set_active(false);
                                }

                                for (i, w) in data.custom_widgets.iter().enumerate() {
                                    d.add_custom_widget(data.items[data.items.len() - custom_widgets_count + i].text.clone(), w.clone());
                                }
                                data.custom_widgets.clear();
                                data.dialog = Some(d);
                            }
                        }
                    }
                }
            }
        });

        Self {
            caption: Text::new(caption),
            editor: LineEdit::new(None),
            button,
            widget_state: WidgetState::default(),
            internal_data,
        }
    }

    pub async fn add_custom_widget(
        &mut self,
        item: Item<T>,
        w: Arc<dyn WidgetTrait>,
        r: Arc<dyn CustomWidgetItemUpdater<T>>,
    ) {
        let mut data = self.internal_data.write().await;
        data.items.push(item);
        data.custom_widgets.push(w);
        data.custom_widget_item_updaters.push(r);
    }

    pub async fn current_item(self, item: &Item<T>) -> Self {
        let mut data = self.internal_data.write().await;
        if data.items.iter().any(|i| i == item) {
            data.selected = Some(item.clone());
        }
        drop(data);
        self
    }

    pub async fn set_items(&self, items: &[Item<T>]) {
        let mut data = self.internal_data.write().await;
        data.items = items.to_vec();
        data.selected = None;
    }

    pub async fn set_current_item(&self, item: &Item<T>) {
        let mut data = self.internal_data.write().await;
        if data.items.iter().any(|i| i == item) {
            data.selected = Some(item.clone());
        }
    }

    pub async fn value(&self) -> Option<Item<T>> {
        self.internal_data.read().await.selected.clone()
    }
}

#[async_trait]
impl<T> WidgetTrait for ComboBox<T>
where
    T: Clone + Send + Sync + 'static,
{
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [mut caption_area, editor_area, button_area] = Layout::horizontal([
            Constraint::Length(self.caption.size().width),
            Constraint::Fill(1),
            Constraint::Length(self.button.size().width),
        ])
        .areas(area);

        caption_area.y += 1; // Center caption vertically
        self.caption.render(caption_area, buf).await;

        self.editor.set_text(
            self.internal_data
                .read()
                .await
                .selected
                .as_ref()
                .map(|item| item.display.clone())
                .unwrap_or_default()
                .as_str(),
        );
        self.editor.render(editor_area, buf).await;
        self.button.render(button_area, buf).await;

        if let Some(d) = &mut self.internal_data.write().await.dialog {
            let size = d.size();
            let area = Rect {
                x: area.x + area.width - size.width,
                y: area.y + self.size().height,
                width: size.width,
                height: size.height,
            };

            Clear {}.render(area, buf);
            d.render(area, buf).await;
        }
    }

    fn size(&self) -> Size {
        Size::new(20, 3)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl<T> KeyboardHandler for ComboBox<T>
where
    T: Clone + Send + Sync,
{
    #[tracing::instrument(level = "debug", target = "handle_keyboard")]
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_active() {
            return false;
        }

        let mut handled = None;
        let mut selected = None;
        let mut should_delete_dialog = false;

        let mut data = self.internal_data.write().await;
        if let Some(d) = &mut data.dialog {
            handled = Some(d.handle_key(key).await);
            if handled.is_some_and(|h| h) && d.should_be_closed() {
                should_delete_dialog = true;
                selected = d.selected().clone();
                data.custom_widgets = d.custom_widgets();
            }
        }

        tracing::debug!(dialog_exists = data.dialog.is_some(), handled = handled, selected=?selected);

        if should_delete_dialog {
            data.dialog = None;
        }

        if let Some(selected) = selected {
            let idx = data.items.iter().position(|item| item.text == selected).unwrap();
            if idx >= data.items.len() - data.custom_widgets.len() {
                let widget_idx = idx - (data.items.len() - data.custom_widgets.len());
                let w = data.custom_widgets[widget_idx].clone();
                let f = data.custom_widget_item_updaters[widget_idx].clone();
                f.update(w, &mut data.items[idx]);
            }
            data.selected = Some(data.items[idx].clone());
        }

        if let Some(handled) = handled {
            return handled;
        }

        return self.button.handle_key(key).await;
    }
}

#[async_trait]
impl<T> MouseHandler for ComboBox<T>
where
    T: Sync + Send,
{
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
