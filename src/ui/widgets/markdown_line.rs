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

pub struct MarkdownLine {
    widgets: ArcRwLock<Vec<Box<dyn WidgetTrait>>>,
}

impl MarkdownLine {
    pub fn new(text: &str) -> Self {
        Self {
            widgets: Arc::new(RwLock::new(
                match markdown::to_mdast(text, &markdown::ParseOptions::default()) {
                    Ok(root) => widgets(&root),
                    Err(_) => Vec::new(),
                },
            )),
        }
    }
}

#[async_trait]
impl WidgetTrait for MarkdownLine {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut area = area;
        for w in self.widgets.write().await.iter_mut() {
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
