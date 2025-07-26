// SPDX-License-Identifier: MIT

use std::{any::Any, sync::Arc};

use super::{
    AppBlockWidget,
    header::Header,
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    shortcut::Shortcut,
    widgets::HyperlinkWidget,
    widgets::{Text, WidgetState, WidgetStateTrait, WidgetTrait},
};
use crate::{
    task::{self, Task as TaskTrait},
    types::ArcRwLock,
    ui::{style, widgets::MarkdownLine},
};
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Modifier, Style, Stylize},
    text::{Line, Text as RatatuiText},
    widgets::{Paragraph, Widget, Wrap},
};
use tokio::sync::RwLock;

struct Entry {
    title: String,
    widget: Box<dyn WidgetTrait>,
}

pub struct TaskInfoWidget {
    t: Option<Box<dyn TaskTrait>>,
    shortcut: Shortcut,
    entries: ArcRwLock<Vec<Entry>>,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(TaskInfoWidget);

impl Default for TaskInfoWidget {
    fn default() -> Self {
        Self {
            t: None,
            shortcut: Shortcut::new("Activate Task Info block", &['g', 'i']),
            entries: Arc::new(RwLock::new(Vec::new())),
            widget_state: WidgetState::default(),
        }
    }
}

#[async_trait]
impl AppBlockWidget for TaskInfoWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        vec![&mut self.shortcut]
    }

    async fn select_next(&mut self) {}
    async fn select_previous(&mut self) {}
    async fn select_first(&mut self) {}
    async fn select_last(&mut self) {}
}

#[async_trait]
impl MouseHandler for TaskInfoWidget {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        for e in self.entries.write().await.iter_mut() {
            e.widget.handle_mouse(ev).await;
        }
    }
}

#[async_trait]
impl KeyboardHandler for TaskInfoWidget {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

impl TaskInfoWidget {
    pub async fn set_task(&mut self, t: Option<Box<dyn TaskTrait>>) {
        self.t = t;

        let mut entries = Vec::new();
        if let Some(t) = &self.t {
            let tz = Local::now().timezone();
            entries.push(Entry {
                title: "ID".to_string(),
                widget: Box::new(Text::new(t.id().as_str())),
            });
            entries.push(Entry {
                title: "Provider".to_string(),
                widget: Box::new(Text::new(t.provider().as_str())),
            });
            entries.push(Entry {
                title: "Text".to_string(),
                widget: Box::new(MarkdownLine::new(t.text().as_str())),
            });

            if let Some(d) = t.due() {
                entries.push(Entry {
                    title: "Due".to_string(),
                    widget: Box::new(Text::new(task::datetime_to_str(Some(d), &tz).as_str())),
                });
            }

            if let Some(d) = t.completed_at() {
                entries.push(Entry {
                    title: "Completed at".to_string(),
                    widget: Box::new(Text::new(task::datetime_to_str(Some(d), &tz).as_str())),
                });
            }

            entries.push(Entry {
                title: "Priority".to_string(),
                widget: Box::new(Text::new(t.priority().to_string().as_str())),
            });

            if let Some(d) = t.description() {
                entries.push(Entry {
                    title: "Description".to_string(),
                    widget: Box::new(MarkdownLine::new(d.as_str())),
                });
            }

            entries.push(Entry {
                title: "Url".to_string(),
                widget: Box::new(HyperlinkWidget::new("Link", t.url().as_str())),
            });

            if let Some(d) = t.created_at() {
                entries.push(Entry {
                    title: "Created at".to_string(),
                    widget: Box::new(Text::new(task::datetime_to_str(Some(d), &tz).as_str())),
                });
            }

            if let Some(d) = t.updated_at() {
                entries.push(Entry {
                    title: "Updated at".to_string(),
                    widget: Box::new(Text::new(task::datetime_to_str(Some(d), &tz).as_str())),
                });
            }

            let value_style = Style::default().fg(style::DESCRIPTION_VALUE_COLOR);
            for e in entries.iter_mut() {
                e.widget.set_style(value_style);
            }
        }

        let mut e = self.entries.write().await;
        e.clear();
        e.extend(entries);
    }
}

#[async_trait]
impl WidgetTrait for TaskInfoWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let h = Header::new("Task info", self.is_active(), Some(&self.shortcut));

        if self.t.is_none() {
            Paragraph::new("Nothing selected...")
                .block(h.block())
                .fg(style::DESCRIPTION_VALUE_COLOR)
                .wrap(Wrap { trim: false })
                .render(area, buf);
            return;
        };

        h.block().render(area, buf);

        let mut row_area = area;
        row_area.y += 1;
        for e in self.entries.write().await.iter_mut() {
            let label = RatatuiText::from(Line::styled(
                format!("{}: ", e.title),
                Style::new()
                    .fg(style::DESCRIPTION_KEY_COLOR)
                    .add_modifier(Modifier::BOLD),
            ));
            let label_width = label.width() as u16;
            label.render(row_area, buf);

            row_area.x += label_width;
            e.widget.set_pos(Position::new(row_area.x, row_area.y));
            e.widget.render(row_area, buf).await;
            row_area.x = area.x;
            row_area.y += e.widget.size().height;
            if row_area.y >= area.y + area.height {
                break;
            }
        }
    }

    fn size(&self) -> Size {
        Size::default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
