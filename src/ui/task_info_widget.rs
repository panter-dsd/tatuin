// SPDX-License-Identifier: MIT

use super::AppBlockWidget;
use crate::task;
use crate::task::Task as TaskTrait;
use crate::ui::style;
use async_trait::async_trait;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use super::header::Header;
use super::shortcut::Shortcut;

pub struct TaskInfoWidget {
    is_active: bool,
    t: Option<Box<dyn TaskTrait>>,
    shortcut: Shortcut,
}

impl Default for TaskInfoWidget {
    fn default() -> Self {
        Self {
            is_active: false,
            t: None,
            shortcut: Shortcut::new(&['g', 'i']),
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

impl TaskInfoWidget {
    pub fn set_task(&mut self, t: Option<Box<dyn TaskTrait>>) {
        self.t = t
    }
}

impl Widget for &mut TaskInfoWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let h = Header::new("Task info", self.is_active, Some(&self.shortcut));

        if let Some(t) = &self.t {
            let id = t.id();
            let task_text = t.text();
            let provider = t.provider();
            let mut text = vec![
                styled_line("ID", &id),
                styled_line("Provider", &provider),
                styled_line("Text", &task_text),
            ];

            let due;
            if t.due().is_some() {
                due = task::datetime_to_str(t.due());
                text.push(styled_line("Due", &due));
            }

            let completed_at;
            if t.completed_at().is_some() {
                completed_at = task::datetime_to_str(t.completed_at());
                text.push(styled_line("Completed at", &completed_at));
            }

            let priority = t.priority().to_string();
            text.push(styled_line("Priority", priority.as_str()));

            let description;
            if let Some(d) = t.description() {
                if !d.is_empty() {
                    description = d;
                    text.push(styled_line("Description", description.as_str()));
                }
            }

            let created_at;
            if let Some(d) = t.created_at() {
                created_at = d.format("%Y-%m-%d %H:%M:%S").to_string();
                text.push(styled_line("Created", &created_at));
            }

            let updated_at;
            if let Some(d) = t.updated_at() {
                updated_at = d.format("%Y-%m-%d %H:%M:%S").to_string();
                text.push(styled_line("Updated", &updated_at));
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
}

fn styled_line<'a>(k: &'a str, v: &'a str) -> Line<'a> {
    let label_style = Style::new()
        .fg(style::DESCRIPTION_KEY_COLOR)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::new().fg(style::DESCRIPTION_VALUE_COLOR);
    Line::from(vec![
        Span::styled(format!("{k}:"), label_style),
        Span::styled(v, value_style),
    ])
}
