use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::task;
use crate::task::Task as TaskTrait;
use crate::ui::selectable_list::SelectableList;
use crate::ui::style;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, ListState, Widget};
use std::cmp::Ordering;
use std::slice::IterMut;

use super::shortcut::Shortcut;

pub struct TasksWidget {
    tasks: SelectableList<Box<dyn TaskTrait>>,
}

impl Default for TasksWidget {
    fn default() -> Self {
        Self {
            tasks: SelectableList::default().shortcut(Shortcut::new(&['g', 't'])),
        }
    }
}

impl TasksWidget {
    pub fn set_active(&mut self, is_active: bool) {
        self.tasks.set_active(is_active)
    }

    pub fn set_tasks(&mut self, tasks: Vec<Box<dyn task::Task>>) {
        self.tasks.set_items(tasks);

        let state = if self.tasks.is_empty() {
            ListState::default()
        } else {
            let selected_idx = self
                .tasks
                .state()
                .selected()
                .map(|i| {
                    if i >= self.tasks.len() {
                        self.tasks.len() - 1
                    } else {
                        i
                    }
                })
                .unwrap_or_else(|| 0);
            ListState::default().with_selected(Some(selected_idx))
        };

        self.tasks.set_state(state);
    }

    pub fn tasks_projects(&self) -> Vec<Box<dyn ProjectTrait>> {
        let mut projects: Vec<Box<dyn ProjectTrait>> = Vec::new();

        for t in self.tasks.iter() {
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
        if let Some(t) = self.tasks.selected() {
            Some(t.clone_boxed())
        } else {
            None
        }
    }

    pub fn select_none(&mut self) {
        self.tasks.select_none();
    }

    pub fn select_next(&mut self) {
        self.tasks.select_next()
    }

    pub fn select_previous(&mut self) {
        self.tasks.select_previous()
    }

    pub fn select_first(&mut self) {
        self.tasks.select_first()
    }

    pub fn select_last(&mut self) {
        self.tasks.select_last()
    }

    pub async fn change_check_state(
        &mut self,
        providers: &mut IterMut<'_, Box<dyn ProviderTrait>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return Ok(());
        }

        let t = selected.unwrap();

        let provider = providers.find(|p| p.name() == t.provider()).unwrap();
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
        self.tasks.render(
            "Tasks",
            |t| {
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
            },
            area,
            buf,
        );
    }
}
