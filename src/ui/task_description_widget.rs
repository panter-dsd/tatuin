use super::AppBlockWidget;
use crate::task;
use crate::task::Task as TaskTrait;
use crate::ui::style;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use super::header::Header;
use super::shortcut::Shortcut;

pub struct TaskDescriptionWidget {
    is_active: bool,
    t: Option<Box<dyn TaskTrait>>,
    shortcut: Option<Shortcut>,
}

impl Default for TaskDescriptionWidget {
    fn default() -> Self {
        Self {
            is_active: false,
            t: None,
            shortcut: Some(Shortcut::new(&['g', 'c'])),
        }
    }
}

impl AppBlockWidget for TaskDescriptionWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        if let Some(s) = &mut self.shortcut {
            vec![s]
        } else {
            Vec::new()
        }
    }

    fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }
}

impl TaskDescriptionWidget {
    pub fn set_task(&mut self, t: Option<Box<dyn TaskTrait>>) {
        self.t = t
    }
}

impl Widget for &mut TaskDescriptionWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let h = Header::new("Task description", self.is_active, &self.shortcut);

        if let Some(t) = &self.t {
            let id = t.id();
            let task_text = t.text();
            let provider = t.provider();
            let mut text = vec![
                styled_line("ID", &id),
                styled_line("Provider", &provider),
                styled_line("Text", &task_text),
            ];

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

            let completed_at;
            if let Some(d) = t.completed_at() {
                completed_at = d.format("%Y-%m-%d %H:%M:%S").to_string();
                text.push(styled_line("Completed", &completed_at));
            }

            let due;
            if t.due().is_some() {
                due = task::due_to_str(t.due());
                text.push(styled_line("Due", &due));
            }

            let priority = t.priority().to_string();
            text.push(styled_line("Priority", priority.as_str()));

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
    let lable_style = Style::new()
        .fg(style::DESCRIPTION_KEY_COLOR)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::new().fg(style::DESCRIPTION_VALUE_COLOR);
    Line::from(vec![
        Span::styled(format!("{k}:"), lable_style),
        Span::styled(v, value_style),
    ])
}
