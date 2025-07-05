// SPDX-License-Identifier: MIT

use super::{MarkdownLine, Text, WidgetTrait};
use crate::{
    task::{self, Task as TaskTrait},
    task_patch::TaskPatch,
    ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style},
};
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Color, Style},
};
use std::cmp::Ordering;

pub struct TaskRow {
    task: Box<dyn TaskTrait>,
    pos: Position,
    children: Vec<Box<dyn WidgetTrait>>,
    is_selected: bool,
    is_visible: bool,
}

impl TaskRow {
    pub fn new(t: &dyn TaskTrait, changed_tasks: &[TaskPatch]) -> Self {
        let tz = Local::now().timezone();

        let fg_color = {
            match t.due() {
                Some(d) => {
                    let now = chrono::Utc::now().date_naive();
                    match d.date_naive().cmp(&now) {
                        Ordering::Less => style::OVERDUE_TASK_FG,
                        Ordering::Equal => style::TODAY_TASK_FG,
                        Ordering::Greater => style::FUTURE_TASK_FG,
                    }
                }
                None => style::NO_DATE_TASK_FG,
            }
        };
        let mut state = t.state();
        let mut due = task::datetime_to_str(t.due(), &tz);
        let mut priority = t.priority();
        let mut uncommitted = false;
        if let Some(patch) = changed_tasks.iter().find(|c| c.is_task(t)) {
            uncommitted = !patch.is_empty();
            if let Some(s) = &patch.state {
                state = s.clone();
            }
            if let Some(d) = &patch.due {
                due = d.to_string();
            }
            if let Some(p) = &patch.priority {
                priority = p.clone();
            }
        }

        let mut children: Vec<Box<dyn WidgetTrait>> = vec![
            Box::new(Text::new(format!("[{state}] ").as_str())),
            Box::new(MarkdownLine::new(t.text().as_str()).style(Style::default().fg(fg_color))),
            Box::new(Text::new(format!(" (due: {due})").as_str()).style(Style::default().fg(Color::Blue))),
            Box::new(
                Text::new(format!(" (Priority: {priority})").as_str())
                    .style(Style::default().fg(style::priority_color(&priority))),
            ),
            Box::new(Text::new(format!(" ({})", t.place()).as_str()).style(Style::default().fg(Color::Yellow))),
        ];

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
            is_visible: true,
        }
    }

    pub fn task(&self) -> &dyn TaskTrait {
        self.task.as_ref()
    }

    pub fn set_selected(&mut self, is_selected: bool) {
        self.is_selected = is_selected
    }

    pub fn set_visible(&mut self, is_visible: bool) {
        self.is_visible = is_visible
    }
}

#[async_trait]
impl WidgetTrait for TaskRow {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let area = Rect {
            x: self.pos.x,
            y: self.pos.y,
            width: area.width,
            height: self.size().height,
        };
        if self.is_selected {
            buf.set_style(area, style::SELECTED_ROW_STYLE);
        } else {
            buf.set_style(area, style::REGULAR_ROW_STYLE);
        }
        for child in self.children.iter_mut() {
            child.render(area, buf).await;
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

    fn pos(&self) -> Position {
        self.pos
    }

    fn set_pos(&mut self, pos: Position) {
        self.pos = pos;
        let mut x = pos.x;

        for child in self.children.iter_mut() {
            child.set_pos(Position::new(x, pos.y));
            x += child.size().width;
        }
    }
}

#[async_trait]
impl KeyboardHandler for TaskRow {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_visible {
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
        if !self.is_visible {
            return;
        }

        for child in self.children.iter_mut() {
            child.handle_mouse(ev).await;
        }
    }
}
