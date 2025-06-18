// SPDX-License-Identifier: MIT

use super::{
    AppBlockWidget, header::Header, hyperlink_widget::HyperlinkWidget, keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler, shortcut::Shortcut, widgets::WidgetTrait,
};
use crate::{
    task::{self, Task as TaskTrait},
    ui::style,
};
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};

pub struct TaskInfoWidget {
    is_active: bool,
    t: Option<Box<dyn TaskTrait>>,
    shortcut: Shortcut,
    url_widget: Option<HyperlinkWidget>,
}

impl Default for TaskInfoWidget {
    fn default() -> Self {
        Self {
            is_active: false,
            t: None,
            shortcut: Shortcut::new("Activate Task Info block", &['g', 'i']),
            url_widget: None,
        }
    }
}

#[async_trait]
impl AppBlockWidget for TaskInfoWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        vec![&mut self.shortcut]
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }

    async fn select_next(&mut self) {}
    async fn select_previous(&mut self) {}
    async fn select_first(&mut self) {}
    async fn select_last(&mut self) {}
}

#[async_trait]
impl MouseHandler for TaskInfoWidget {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        if let Some(w) = &mut self.url_widget {
            w.handle_mouse(ev).await;
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
    pub fn set_task(&mut self, t: Option<Box<dyn TaskTrait>>) {
        self.t = t;

        if let Some(t) = &self.t {
            let url = t.url();
            if !url.is_empty() {
                self.url_widget
                    .replace(HyperlinkWidget::new("Open in Obsidian", url.as_str()));
            }
        }
    }
}

#[async_trait]
impl WidgetTrait for TaskInfoWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let h = Header::new("Task info", self.is_active, Some(&self.shortcut));
        let tz = Local::now().timezone();

        if let Some(t) = &self.t {
            let id = t.id();
            let task_text = t.text();
            let provider = t.provider();
            let mut text = vec![
                styled_line("ID", &id, None),
                styled_line("Provider", &provider, None),
                styled_line("Text", &task_text, None),
            ];

            let due;
            if t.due().is_some() {
                due = task::datetime_to_str(t.due(), &tz);
                text.push(styled_line("Due", &due, None));
            }

            let completed_at;
            if t.completed_at().is_some() {
                completed_at = task::datetime_to_str(t.completed_at(), &tz);
                text.push(styled_line("Completed at", &completed_at, None));
            }

            let priority = t.priority().to_string();
            text.push(styled_line("Priority", priority.as_str(), None));

            let description;
            if let Some(d) = t.description() {
                if !d.is_empty() {
                    description = d;
                    text.push(styled_line("Description", description.as_str(), None));
                }
            }

            if let Some(w) = &mut self.url_widget {
                const LABEL: &str = "Url";

                w.set_pos(
                    area,
                    Position::new(
                        area.x + Text::from(LABEL).width() as u16 + 1,
                        area.y + text.len() as u16 + 1,
                    ),
                );
                text.push(styled_line(LABEL, "", None));
                w.render(buf);
            }

            let created_at;
            if t.created_at().is_some() {
                created_at = task::datetime_to_str(t.created_at(), &tz);
                text.push(styled_line("Created", &created_at, None));
            }

            let updated_at;
            if t.updated_at().is_some() {
                updated_at = task::datetime_to_str(t.updated_at(), &tz);
                text.push(styled_line("Updated", &updated_at, None));
            }

            Paragraph::new(text)
                .block(h.block())
                .wrap(Wrap { trim: false })
                .render(area, buf);
        } else {
            Paragraph::new("Nothing selected...")
                .block(h.block())
                .fg(style::DESCRIPTION_VALUE_COLOR)
                .wrap(Wrap { trim: false })
                .render(area, buf);
        };
    }

    fn size(&self) -> Size {
        Size::default()
    }
}

fn styled_line<'a>(k: &'a str, v: &'a str, modifier: Option<fn(&Style) -> Style>) -> Line<'a> {
    let label_style = Style::new()
        .fg(style::DESCRIPTION_KEY_COLOR)
        .add_modifier(Modifier::BOLD);
    let mut value_style = Style::new().fg(style::DESCRIPTION_VALUE_COLOR);
    if let Some(m) = modifier {
        value_style = m(&value_style);
    }
    Line::from(vec![
        Span::styled(format!("{k}:"), label_style),
        Span::styled(v, value_style),
    ])
}
