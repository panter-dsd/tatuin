// SPDX-License-Identifier: MIT

use super::{MarkdownLine, Text, WidgetState, WidgetStateTrait, WidgetTrait};
use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};
use async_trait::async_trait;
use chrono::{Local, NaiveTime};
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::Style,
};
use std::{any::Any, cmp::Ordering};
use tatuin_core::{
    task::{self, Task as TaskTrait},
    task_patch::TaskPatch,
};

pub struct TaskRow {
    task: Box<dyn TaskTrait>,
    pos: Position,
    children: Vec<Box<dyn WidgetTrait>>,
    is_selected: bool,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(TaskRow);

impl TaskRow {
    pub fn new(t: &dyn TaskTrait, changed_tasks: &[TaskPatch]) -> Self {
        let tz = Local::now().timezone();

        let fg_color = {
            match t.due() {
                Some(d) => {
                    let now = chrono::Utc::now();
                    match d.date_naive().cmp(&now.date_naive()) {
                        Ordering::Less => style::overdue_task_fg(),
                        Ordering::Equal => {
                            if d.time() == NaiveTime::default() {
                                style::today_task_fg()
                            } else {
                                match d.cmp(&now) {
                                    Ordering::Less => style::overdue_task_fg(),
                                    Ordering::Equal => style::today_task_fg(),
                                    Ordering::Greater => style::future_task_fg(),
                                }
                            }
                        }
                        Ordering::Greater => style::future_task_fg(),
                    }
                }
                None => style::no_date_task_fg(),
            }
        };
        let mut name = t.text();
        let mut state = t.state();
        let mut due = task::datetime_to_str(t.due(), &tz);
        let mut priority = t.priority();
        let mut uncommitted = false;
        if let Some(patch) = changed_tasks.iter().find(|c| c.is_task(t)) {
            uncommitted = !patch.is_empty();
            if let Some(n) = &patch.name.value() {
                name = n.to_string();
            }
            if let Some(s) = &patch.state.value() {
                state = *s;
            }
            if let Some(d) = &patch.due.value() {
                due = d.to_string();
            }
            if let Some(p) = &patch.priority.value() {
                priority = *p;
            }
        }

        let mut children: Vec<Box<dyn WidgetTrait>> = vec![
            Box::new(Text::new(format!("[{state}] ").as_str())),
            Box::new(MarkdownLine::new(name.as_str()).style(style::default_style().fg(fg_color))),
            Box::new(Text::new(format!(" (due: {due})").as_str()).style(style::default_style().fg(style::due_color()))),
            Box::new(
                Text::new(format!(" (Priority: {priority})").as_str())
                    .style(style::default_style().fg(style::priority_color(&priority))),
            ),
            Box::new(
                Text::new(format!(" ({})", t.place()).as_str()).style(style::default_style().fg(style::place_color())),
            ),
        ];

        for l in t.labels() {
            children.push(Box::new(Text::new(" ")));
            children.push(Box::new(
                Text::new(format!("🏷️{l}").as_str()).style(style::label_style()),
            ));
        }

        if !t.description().unwrap_or_default().is_empty() {
            children.push(Box::new(Text::new(" 💬")));
        }

        if uncommitted {
            children.push(Box::new(Text::new(" 📤")));
        }

        Self {
            task: t.clone_boxed(),
            children,
            pos: Position::default(),
            is_selected: false,
            widget_state: WidgetState::default(),
        }
    }

    pub fn task(&self) -> &dyn TaskTrait {
        self.task.as_ref()
    }

    pub fn set_selected(&mut self, is_selected: bool) {
        self.is_selected = is_selected
    }
}

#[async_trait]
impl WidgetTrait for TaskRow {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut area = Rect {
            x: self.pos.x,
            y: self.pos.y,
            width: area.width,
            height: self.size().height,
        };

        let mut s = if self.is_selected {
            buf.set_style(area, style::selected_row_style());
            style::selected_row_style()
        } else {
            buf.set_style(area, style::regular_row_style());
            style::regular_row_style()
        };
        s.fg = None;
        for child in self.children.iter_mut() {
            child.set_style(child.style().patch(s));
            child.render(area, buf).await;
            area.x += child.size().width;
        }
    }

    fn size(&self) -> Size {
        let mut result = Size::default();

        for child in self.children.iter() {
            result.width += child.size().width;
            result.height = result.height.max(child.size().height);
        }

        result
    }

    fn set_style(&mut self, style: Style) {
        for child in self.children.iter_mut() {
            let mut s = child.style();
            s.bg = None;
            child.set_style(s.patch(style));
        }
    }

    fn set_pos(&mut self, pos: Position) {
        self.pos = pos;
        let mut x = pos.x;

        for child in self.children.iter_mut() {
            child.set_pos(Position::new(x, pos.y));
            x += child.size().width;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for TaskRow {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_visible() {
            return false;
        }

        for child in self.children.iter_mut() {
            if child.handle_key(key).await {
                return true;
            }
        }

        false
    }
}

#[async_trait]
impl MouseHandler for TaskRow {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        if !self.is_visible() {
            return;
        }

        for child in self.children.iter_mut() {
            child.handle_mouse(ev).await;
        }
    }
}
