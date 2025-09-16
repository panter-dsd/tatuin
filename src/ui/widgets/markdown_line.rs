// SPDX-License-Identifier: MIT

use std::{any::Any, sync::Arc};

use super::{HyperlinkWidget, Text, WidgetState, WidgetStateTrait, WidgetTrait};
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Modifier, Style},
};

use markdown::mdast::Node;
use tokio::sync::RwLock;

use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};
use tatuin_core::types::ArcRwLock;

pub struct MarkdownLine {
    pos: Position,
    width: u16,
    style: Option<Style>,
    style_applied: bool,
    widgets: ArcRwLock<Vec<Box<dyn WidgetTrait>>>,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(MarkdownLine);

fn first_not_empty_string(s: &str) -> String {
    if s.contains('\n')
        && let Some(ss) = s.split('\n').find(|s| !s.trim().is_empty())
    {
        return ss.trim().to_string() + "...";
    }

    s.to_string()
}

impl MarkdownLine {
    pub fn new(text: &str) -> Self {
        let widgets = match markdown::to_mdast(&first_not_empty_string(text), &markdown::ParseOptions::default()) {
            Ok(root) => widgets(&root),
            Err(_) => Vec::new(),
        };
        Self {
            pos: Position::default(),
            width: widgets
                .iter()
                .map(|w| w.size().width)
                .reduce(|acc, w| acc + w)
                .unwrap_or_default(),
            style: None,
            style_applied: true,
            widgets: Arc::new(RwLock::new(widgets)),
            widget_state: WidgetState::default(),
        }
    }

    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self.style_applied = false;
        self
    }
}

#[async_trait]
impl WidgetTrait for MarkdownLine {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.style_applied {
            if let Some(s) = &self.style {
                for w in self.widgets.write().await.iter_mut() {
                    let mut style = w.style();
                    style.bg = s.bg;
                    style.fg = s.fg;
                    w.set_style(style);
                }
            }
            self.style_applied = true;
        }

        let mut area = Rect {
            x: self.pos.x,
            y: self.pos.y,
            width: area.width,
            height: area.height,
        };

        for w in self.widgets.write().await.iter_mut() {
            let size = w.size();
            w.set_pos(Position::new(area.x, area.y));
            w.render(area, buf).await;
            area.x += size.width;
        }
    }

    fn size(&self) -> Size {
        Size::new(self.width, 1)
    }

    fn set_pos(&mut self, pos: Position) {
        self.pos = pos
    }

    fn style(&self) -> Style {
        self.style.unwrap_or(style::default_style())
    }

    fn set_style(&mut self, style: Style) {
        let mut style = style;
        if let Some(s) = &mut self.style {
            style.fg = s.fg;
        }
        self.style = Some(style);
        self.style_applied = false;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for MarkdownLine {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

#[async_trait]
impl MouseHandler for MarkdownLine {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        for h in self.widgets.write().await.iter_mut() {
            h.handle_mouse(ev).await;
        }
    }
}

fn widgets(node: &Node) -> Vec<Box<dyn WidgetTrait>> {
    let mut result = Vec::new();
    if node.children().is_none() {
        return result;
    }

    for n in node.children().unwrap() {
        match n {
            Node::Text(t) => {
                result.push(Box::new(Text::new(t.value.as_str())));
            }
            Node::Root(_) | Node::Paragraph(_) => {
                result.extend(widgets(n));
            }
            Node::Link(l) => {
                result.push(Box::new(HyperlinkWidget::new(generate_node_text(n).as_str(), &l.url)));
            }
            Node::Strong(_) | Node::Heading(_) => {
                result.push(Box::new(
                    Text::new(generate_node_text(n).as_str()).modifier(Modifier::BOLD),
                ));
            }
            Node::Emphasis(_) => {
                result.push(Box::new(
                    Text::new(generate_node_text(n).as_str()).modifier(Modifier::ITALIC),
                ));
            }
            Node::InlineCode(n) => {
                result.push(Box::new(
                    Text::new(n.value.as_str()).style(style::inline_code_text_style()),
                ));
            }
            Node::Delete(_) => {
                result.push(Box::new(
                    Text::new(generate_node_text(n).as_str()).modifier(Modifier::CROSSED_OUT),
                ));
            }
            _ => {}
        }
    }

    result
}

fn generate_node_text(root: &Node) -> String {
    let mut lines = Vec::new();
    for node in root.children().unwrap() {
        match node {
            Node::Text(t) => lines.push(t.value.clone()),
            Node::Emphasis(_) | Node::Strong(_) => lines.push(generate_node_text(node)),
            Node::InlineCode(t) => lines.push(t.value.clone()),
            _ => {}
        }
    }
    lines.join(" ")
}
