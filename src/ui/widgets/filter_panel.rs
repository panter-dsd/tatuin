use std::any::Any;

use crate::ui::style;
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::ui::{
    WidgetStateTrait, keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, widgets::WidgetState,
};

use super::widget::WidgetTrait;

pub struct Panel {
    tag_filter: Vec<String>,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(Panel);

impl Panel {
    pub fn new() -> Self {
        Self {
            tag_filter: Vec::new(),
            widget_state: WidgetState::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tag_filter.is_empty()
    }

    pub fn tag_filter(&self) -> Vec<String> {
        self.tag_filter.clone()
    }

    pub fn set_tag_filter(&mut self, filter: &[String]) {
        self.tag_filter = filter.to_vec()
    }
}

#[async_trait]
impl WidgetTrait for Panel {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let panel_style = style::default_style();
        let mut lines = Vec::new();

        if !self.tag_filter.is_empty() {
            let and_more_text = |count| format!(", and {} more...", count);
            let and_more_width = Text::from(and_more_text(self.tag_filter.len())).width();

            let mut spans = vec![Span::styled(format!("{} ", style::tag_icon()), panel_style)];
            let mut line_width = spans.first().unwrap().width();
            let max_width = area.width as usize;

            for (i, t) in self.tag_filter.iter().enumerate() {
                let s = Span::styled(
                    if spans.len() == 1 { t.clone() } else { format!(", {t}") },
                    style::label_style(),
                );
                if line_width + s.width() + and_more_width < max_width {
                    line_width += s.width();
                    spans.push(s);
                } else {
                    spans.push(Span::styled(
                        and_more_text(self.tag_filter.len() - i),
                        style::label_style(),
                    ));
                    break;
                }
            }

            lines.push(Line::from(spans));
        }

        let mut r = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 2,
            height: 1,
        };

        for l in lines {
            Paragraph::new(l).render(r, buf);
            r.y += 1;
        }

        Block::default()
            .borders(Borders::ALL)
            .border_style(style::filter_panel_bg())
            .title("Filter")
            .render(area, buf);
    }

    fn min_size(&self) -> Size {
        let mut height: u16 = 2;

        if !self.tag_filter.is_empty() {
            height += 1;
        }

        Size::new(0, height)
    }

    fn size(&self) -> Size {
        self.min_size()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Panel {
    async fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

#[async_trait]
impl MouseHandler for Panel {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
