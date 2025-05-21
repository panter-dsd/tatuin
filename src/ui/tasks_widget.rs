// SPDX-License-Identifier: MIT

use super::AppBlockWidget;
use crate::filter::Filter;
use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::state::StatefulObject;
use crate::task;
use crate::task::{Task as TaskTrait, due_group};
use crate::ui::selectable_list::SelectableList;
use crate::ui::style;
use async_trait::async_trait;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, ListState};
use std::cmp::Ordering;
use std::error::Error;
use std::slice::IterMut;

use super::shortcut::Shortcut;

pub struct TasksWidget {
    all_tasks: Vec<Box<dyn TaskTrait>>,
    tasks: SelectableList<Box<dyn TaskTrait>>,
    providers_filter: Vec<String>,
    projects_filter: Vec<String>,
}

impl Default for TasksWidget {
    fn default() -> Self {
        Self {
            all_tasks: Vec::new(),
            tasks: SelectableList::default().shortcut(Shortcut::new(&['g', 't'])),
            projects_filter: Vec::new(),
            providers_filter: Vec::new(),
        }
    }
}

#[async_trait]
impl AppBlockWidget for TasksWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        self.tasks.activate_shortcuts()
    }

    fn set_active(&mut self, is_active: bool) {
        self.tasks.set_active(is_active);
    }

    async fn select_next(&mut self) {
        self.tasks.select_next().await;
    }

    async fn select_previous(&mut self) {
        self.tasks.select_previous().await;
    }

    async fn select_first(&mut self) {
        self.tasks.select_first().await;
    }

    async fn select_last(&mut self) {
        self.tasks.select_last().await;
    }
}

impl TasksWidget {
    pub fn set_providers_filter(&mut self, providers: &[String]) {
        self.providers_filter = providers.to_vec();
        self.filter_tasks();
    }

    pub fn set_projects_filter(&mut self, projects: &[String]) {
        self.projects_filter = projects.to_vec();
        self.filter_tasks();
    }

    fn filter_tasks(&mut self) {
        self.tasks.set_items(
            self.all_tasks
                .iter()
                .filter(|t| {
                    let mut result = true;
                    if !self.providers_filter.is_empty() && !self.providers_filter.contains(&t.provider()) {
                        result = false;
                    }
                    if let Some(tp) = t.project() {
                        if !self.projects_filter.is_empty() && !self.projects_filter.contains(&tp.name()) {
                            result = false;
                        }
                    }
                    result
                })
                .map(|t| t.clone_boxed())
                .collect(),
        );
        let state = if self.all_tasks.is_empty() {
            ListState::default()
        } else {
            let selected_idx = self
                .tasks
                .state()
                .selected()
                .map(|i| {
                    if i >= self.all_tasks.len() {
                        self.all_tasks.len() - 1
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

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
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
                let mut lines = vec![
                    Span::from(format!("[{}] ", t.state())),
                    Span::styled(t.text(), Style::default().fg(fg_color)),
                    Span::from(" ("),
                    Span::styled(
                        format!("due: {}", task::datetime_to_str(t.due())),
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
                ];

                if !t.description().unwrap_or_default().is_empty() {
                    lines.push(Span::from(" 💬"));
                }

                ListItem::from(Line::from(lines))
            },
            area,
            buf,
        );
    }

    pub async fn load_tasks(
        &mut self,
        providers: &mut IterMut<'_, Box<dyn ProviderTrait>>,
        f: &Filter,
    ) -> Vec<Box<dyn Error>> {
        let mut all_tasks: Vec<Box<dyn task::Task>> = Vec::new();

        let mut errors = Vec::new();

        for p in providers {
            let tasks = p.tasks(None, f).await;

            match tasks {
                Ok(t) => all_tasks.append(&mut t.iter().map(|t| t.clone_boxed()).collect::<Vec<Box<dyn TaskTrait>>>()),
                Err(err) => errors.push((p.name(), err)),
            }
        }

        all_tasks.sort_by(|l, r| {
            due_group(l.as_ref())
                .cmp(&due_group(r.as_ref()))
                .then_with(|| r.priority().cmp(&l.priority()))
                .then_with(|| l.due().cmp(&r.due()))
        });

        self.all_tasks = all_tasks;
        self.filter_tasks();

        errors
            .iter()
            .map(|(provider_name, err)| {
                Box::<dyn Error>::from(format!("Load provider {provider_name} projects failure: {err}"))
            })
            .collect()
    }
}

impl StatefulObject for TasksWidget {
    fn save(&self) -> crate::state::State {
        self.tasks.save()
    }

    fn restore(&mut self, state: crate::state::State) {
        self.tasks.restore(state);
    }
}
