// SPDX-License-Identifier: MIT

use std::{any::Any, sync::Arc};

use super::{HyperlinkWidget, Text, WidgetState, WidgetStateTrait, WidgetTrait};
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    style::{Modifier, Style},
};

use markdown::mdast::Node;
use tokio::sync::RwLock;

use crate::ui::{keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, style};
use tatuin_core::types::ArcRwLock;

pub struct Config {
    pub skip_first_empty_lines: bool,
    pub line_count: usize,
}

impl Config {
    pub fn default() -> Self {
        Self {
            skip_first_empty_lines: true,
            line_count: 1,
        }
    }

    fn apply(&self, text: &str) -> String {
        let mut text = text.to_string();
        if self.skip_first_empty_lines {
            text = skip_empty_lines_at_start(text.as_str());
        }

        let t = text.split("\n").take(self.line_count).join("\n");
        if text != t { t + "..." } else { text.to_string() }
    }
}

type Line = Vec<Box<dyn WidgetTrait>>;

pub struct MarkdownView {
    pos: Position,
    width: u16,
    height: u16,
    style: Option<Style>,
    style_applied: bool,
    lines: ArcRwLock<Vec<Line>>,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(MarkdownView);

impl MarkdownView {
    pub fn new(text: &str, cfg: Config) -> Self {
        let text = cfg.apply(text);

        let mut lines = Vec::new();
        let mut width = 0;
        let mut height = 0;
        for t in text.split("\n") {
            let line = match markdown::to_mdast(t, &markdown::ParseOptions::default()) {
                Ok(root) => widgets(&root),
                Err(_) => Vec::new(),
            };

            width = width.max(
                line.iter()
                    .map(|w| w.size().width)
                    .reduce(|acc, w| acc + w)
                    .unwrap_or_default(),
            );
            height += 1;
            lines.push(line);
        }

        Self {
            pos: Position::default(),
            width,
            height,
            style: None,
            style_applied: true,
            lines: Arc::new(RwLock::new(lines)),
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
impl WidgetTrait for MarkdownView {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.style_applied {
            if let Some(s) = &self.style {
                for line in self.lines.write().await.iter_mut() {
                    for w in line {
                        let mut style = w.style();
                        style.bg = s.bg;
                        style.fg = s.fg;
                        w.set_style(style);
                    }
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

        for line in self.lines.write().await.iter_mut() {
            let mut line_area = area;
            for w in line {
                let size = w.size();
                w.set_pos(Position::new(line_area.x, line_area.y));
                w.render(line_area, buf).await;
                line_area.x += size.width;
            }
            area.y += 1;
        }
    }

    fn size(&self) -> Size {
        Size::new(self.width, self.height)
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
impl KeyboardHandler for MarkdownView {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

#[async_trait]
impl MouseHandler for MarkdownView {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        for line in self.lines.write().await.iter_mut() {
            for w in line {
                w.handle_mouse(ev).await;
            }
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

fn skip_empty_lines_at_start(s: &str) -> String {
    s.split("\n").skip_while(|s| s.trim().is_empty()).join("\n")
}

#[cfg(test)]
mod test {
    use super::skip_empty_lines_at_start;

    #[test]
    fn skip_empty_lines_at_start_test() {
        struct Case<'a> {
            name: &'a str,
            input: &'a str,
            output: &'a str,
        }
        const CASES: &[Case] = &[
            Case {
                name: "empty string",
                input: "",
                output: "",
            },
            Case {
                name: "string with one line without symbols",
                input: " ",
                output: "",
            },
            Case {
                name: "string with one line with symbols",
                input: " some text ",
                output: " some text ",
            },
        ];

        for c in CASES {
            let s = skip_empty_lines_at_start(c.input);
            assert_eq!(c.output, s.as_str(), "Test '{}' was failed", c.name)
        }
    }
}
