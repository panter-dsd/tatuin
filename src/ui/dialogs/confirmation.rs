// SPDX-License-Identifier: MIT

use std::any::Any;

use super::DialogTrait;
use crate::ui::{
    draw_helper::DrawHelper,
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    order_changer::OrderChanger,
    style,
    widgets::{Button, WidgetState, WidgetStateTrait, WidgetTrait},
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect, Size},
    text::Text,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Copy, Clone, PartialEq, Eq, strum::Display)]
#[allow(dead_code)]
pub enum StandardButton {
    Ok,
    Yes,
    No,
    Cancel,
}

struct DialogButton {
    standard_button: StandardButton,
    widget: Button,
}

#[allow(dead_code)]
pub enum Icon {
    Question,
    Warning,
    Error,
    Custom(char),
}

pub struct Dialog {
    title: String,
    icon: Option<Icon>,
    question: String,
    buttons: Vec<DialogButton>,
    choice: Option<StandardButton>,
    should_be_closed: bool,
    widget_state: WidgetState,
}
crate::impl_widget_state_trait!(Dialog);

impl Dialog {
    pub fn new(title: &str, question: &str, buttons: &[StandardButton], default_button: StandardButton) -> Self {
        let buttons = buttons
            .iter()
            .map(|b| {
                let mut button = Button::new(b.to_string().as_str());
                button.set_active(b == &default_button);
                DialogButton {
                    standard_button: *b,
                    widget: button,
                }
            })
            .collect();
        Self {
            title: title.to_string(),
            icon: None,
            question: question.to_string(),
            buttons,
            choice: None,
            should_be_closed: false,
            widget_state: WidgetState::default(),
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn is_confirmed(&self) -> bool {
        if self.choice.is_none() {
            return false;
        }
        match self.choice.unwrap() {
            StandardButton::Ok => true,
            StandardButton::Yes => true,
            StandardButton::No => false,
            StandardButton::Cancel => false,
        }
    }

    fn order_calculator(&mut self) -> OrderChanger<'_> {
        OrderChanger::new(
            self.buttons
                .iter_mut()
                .map(|b| &mut b.widget as &'_ mut dyn WidgetTrait)
                .collect(),
        )
    }

    async fn next_widget(&mut self) {
        self.order_calculator().select_next();
    }

    async fn prev_widget(&mut self) {
        self.order_calculator().select_prev();
    }
}

#[async_trait]
impl WidgetTrait for Dialog {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let title = match self.icon {
            Some(Icon::Question) => format!("❔ {}", self.title),
            Some(Icon::Warning) => format!("⚠️ {}", self.title),
            Some(Icon::Error) => format!("❌ {}", self.title),
            Some(Icon::Custom(c)) => format!("{c} {}", self.title),
            None => self.title.clone(),
        };

        let b = Block::default()
            .title_top(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(style::border_color());

        let [question_area, buttons_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(b.inner(area));
        b.render(area, buf);

        Paragraph::new(self.question.as_str())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(question_area, buf);
        let all_buttons_width: u16 = self
            .buttons
            .iter()
            .map(|b| b.widget.size().width + 1 /*separate*/)
            .sum();
        let mut offset = (buttons_area.width - all_buttons_width) / 2;
        for b in &mut self.buttons {
            let w = b.widget.size().width;
            b.widget
                .render(
                    Rect {
                        x: buttons_area.x + offset,
                        y: buttons_area.y,
                        width: w,
                        height: buttons_area.height,
                    },
                    buf,
                )
                .await;
            offset += w + 1;
        }
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        for b in &mut self.buttons {
            b.widget.set_draw_helper(dh.clone());
        }
    }

    fn size(&self) -> Size {
        let width = Text::raw(&self.title).width().max(Text::raw(&self.question).width());
        let mut height = self.question.chars().filter(|c| *c == '\n').count() as u16 + 1;
        height += 1; // empty line
        height += self
            .buttons
            .iter()
            .map(|db| db.widget.size().height)
            .max()
            .unwrap_or_default();

        Size::new(width as u16 + 2, height + 2)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl DialogTrait for Dialog {
    fn should_be_closed(&self) -> bool {
        self.should_be_closed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl KeyboardHandler for Dialog {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        for b in &mut self.buttons {
            if b.widget.handle_key(key).await {
                self.should_be_closed = true;
                self.choice = Some(b.standard_button);
                return true;
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.should_be_closed = true;
                self.choice = Some(StandardButton::Cancel);
            }
            KeyCode::Tab => {
                self.next_widget().await;
            }
            KeyCode::BackTab => {
                self.prev_widget().await;
            }
            _ => {}
        }

        true
    }
}

#[async_trait]
impl MouseHandler for Dialog {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
