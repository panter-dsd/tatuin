use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::task;
use crate::task::Task as TaskTrait;
use crate::ui::selectable_list::SelectableList;
use crate::ui::style;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, StatefulWidget, Widget,
};
use std::cmp::Ordering;

#[derive(Default)]
pub struct TasksWidget {
    is_active: bool,
    tasks: SelectableList<Box<dyn task::Task>>,
}

impl TasksWidget {
    pub fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }

    fn block_style(&self) -> Style {
        if self.is_active {
            return style::ACTIVE_BLOCK_STYLE;
        }

        style::INACTIVE_BLOCK_STYLE
    }

    pub fn set_tasks(&mut self, tasks: Vec<Box<dyn task::Task>>) {
        self.tasks.items = tasks;

        self.tasks.state = if self.tasks.items.is_empty() {
            ListState::default()
        } else {
            let selected_idx = self
                .tasks
                .state
                .selected()
                .map(|i| {
                    if i >= self.tasks.items.len() {
                        self.tasks.items.len() - 1
                    } else {
                        i
                    }
                })
                .unwrap_or_else(|| 0);
            ListState::default().with_selected(Some(selected_idx))
        };
    }

    pub fn tasks_projects(&self) -> Vec<Box<dyn ProjectTrait>> {
        let mut projects: Vec<Box<dyn ProjectTrait>> = Vec::new();

        for t in &self.tasks.items {
            if let Some(tp) = t.project() {
                let it = projects
                    .iter()
                    .find(|p| p.id() == tp.id() && p.provider() == tp.provider());
                if it.is_none() {
                    projects.push(tp.clone_boxed());
                }
            }
        }

        projects
    }

    pub fn selected_task(&self) -> Option<Box<dyn TaskTrait>> {
        if self.tasks.state.selected().is_some() && !self.tasks.items.is_empty() {
            let selected_idx = std::cmp::min(
                self.tasks.state.selected().unwrap_or_default(),
                self.tasks.items.len() - 1,
            );
            let t = &self.tasks.items[selected_idx];
            Some(t.clone_boxed())
        } else {
            None
        }
    }

    pub fn select_none(&mut self) {
        self.tasks.state.select(None)
    }

    pub fn select_next(&mut self) {
        self.tasks.state.select_next()
    }

    pub fn select_previous(&mut self) {
        self.tasks.state.select_previous()
    }

    pub fn select_first(&mut self) {
        self.tasks.state.select_first()
    }

    pub fn select_last(&mut self) {
        self.tasks.state.select_last()
    }

    pub async fn change_check_state(
        &mut self,
        providers: &mut [Box<dyn ProviderTrait>],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.tasks.state.selected().is_none() {
            return Ok(());
        }

        let t = &self.tasks.items[self.tasks.state.selected().unwrap()];
        let provider_idx = providers
            .iter()
            .position(|p| p.name() == t.provider())
            .unwrap();
        let provider = &mut providers[provider_idx];
        let st = match t.state() {
            task::State::Completed => task::State::Uncompleted,
            task::State::Uncompleted | task::State::InProgress => task::State::Completed,
            task::State::Unknown(_) => task::State::Completed,
        };

        provider.change_task_state(t.as_ref(), st).await
    }
}

impl Widget for &mut TasksWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .tasks
            .items
            .iter()
            .map(|t| {
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
                let mixed_line = Line::from(vec![
                    Span::from(format!("[{}] ", t.state())),
                    Span::styled(t.text(), Style::default().fg(fg_color)),
                    Span::from(" ("),
                    Span::styled(
                        format!("due: {}", task::due_to_str(t.due())),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::from(") ("),
                    Span::styled(
                        format!("Priority: {}", t.priority()),
                        style::priority_color(&t.priority()),
                    ),
                    Span::from(") ("),
                    Span::styled(t.place(), Style::default().fg(Color::Yellow)),
                    Span::from(")"),
                ]);

                ListItem::from(mixed_line)
            })
            .collect();

        let block = Block::new()
            .title(Line::raw(format!("Tasks ({})", items.len())).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(self.block_style())
            .bg(style::NORMAL_ROW_BG);

        let list = List::new(items.to_vec())
            .block(block)
            .highlight_style(style::SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(list, area, buf, &mut self.tasks.state);
    }
}
