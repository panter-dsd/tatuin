use std::sync::Arc;

use super::{HyperlinkWidget, Text};
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
};

use markdown::mdast::Node;
use tokio::sync::RwLock;

use crate::types::ArcRwLock;
use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler};

use super::WidgetTrait;

fn widgets(node: &Node) -> Vec<ArcRwLock<Box<dyn WidgetTrait>>> {
    let mut result = Vec::new();
    if node.children().is_none() {
        return result;
    }

    for n in node.children().unwrap() {
        match n {
            Node::Text(t) => {
                result.push(Arc::new(RwLock::new(Box::new(Text::new(t.value.as_str())))));
            }
            Node::Root(_) | Node::Paragraph(_) => {
                result.extend(widgets(n));
            }
            Node::Link(l) => {
                result.push(Arc::new(RwLock::new(Box::new(HyperlinkWidget::new(
                    generate_node_text(n).as_str(),
                    &l.url,
                )))));
            }
            _ => {}
        }
    }

    result
}

pub struct MarkdownLine {
    widgets: Vec<ArcRwLock<Box<dyn WidgetTrait>>>,
}

impl MarkdownLine {
    pub fn new(text: &str) -> Self {
        Self {
            widgets: match markdown::to_mdast(text, &markdown::ParseOptions::default()) {
                Ok(root) => widgets(&root),
                Err(_) => Vec::new(),
            },
        }
    }
}

#[async_trait]
impl WidgetTrait for MarkdownLine {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut area = area;
        for w in &self.widgets {
            let mut w = w.write().await;
            w.set_pos(Position::new(area.x, area.y));
            w.render(area, buf).await;
            area.x += w.size().width;
        }
    }

    fn size(&self) -> Size {
        Size::new(30, 1)
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
        for h in &self.widgets {
            h.write().await.handle_mouse(ev).await;
        }
    }
}

fn generate_node_text(root: &markdown::mdast::Node) -> String {
    let mut lines = Vec::new();
    for node in root.children().unwrap() {
        match node {
            markdown::mdast::Node::Text(t) => {
                lines.push(t.value.clone());
            }
            _ => {}
        }
    }
    lines.join(" ")
}
