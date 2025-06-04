// SPDX-License-Identifier: MIT

use super::AppBlockWidget;
use super::mouse_handler::MouseHandler;
use crate::filter::Filter;
use crate::project::Project as ProjectTrait;
use crate::provider::{Provider as ProviderTrait, TaskPatch};
use crate::state::StatefulObject;
use crate::task;
use crate::task::{State, Task as TaskTrait, due_group, equal};
use crate::ui::selectable_list::SelectableList;
use crate::ui::style;
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::MouseEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, ListState};
use std::cmp::Ordering;
use std::slice::IterMut;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::shortcut::Shortcut;

pub trait ProvidersStorage<T>: Send + Sync {
    fn iter_mut(&mut self) -> IterMut<'_, T>;
}

pub trait ErrorLoggerTrait: Send + Sync {
    fn add_error(&mut self, message: &str);
    fn add_errors(&mut self, messages: &[&str]);
}

type ErrorLogger = Arc<RwLock<dyn ErrorLoggerTrait>>;

struct ChangedState {
    task: Box<dyn TaskTrait>,
    new_state: State,
}

pub struct TasksWidget {
    providers_storage: Arc<RwLock<dyn ProvidersStorage<Box<dyn ProviderTrait>>>>,
    error_logger: ErrorLogger,
    all_tasks: Vec<Box<dyn TaskTrait>>,
    changed_state_tasks: Vec<ChangedState>,
    tasks: SelectableList<Box<dyn TaskTrait>>,
    providers_filter: Vec<String>,
    projects_filter: Vec<String>,

    commit_changes_shortcut: Shortcut,
    swap_completed_state_shortcut: Shortcut,
    in_progress_shortcut: Shortcut,
    last_filter: Filter,
}

#[async_trait]
impl AppBlockWidget for TasksWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        self.tasks.activate_shortcuts()
    }
    fn shortcuts(&mut self) -> Vec<&mut Shortcut> {
        vec![
            &mut self.commit_changes_shortcut,
            &mut self.swap_completed_state_shortcut,
            &mut self.in_progress_shortcut,
        ]
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
    pub fn new(
        providers_storage: Arc<RwLock<dyn ProvidersStorage<Box<dyn ProviderTrait>>>>,
        error_logger: ErrorLogger,
    ) -> Arc<RwLock<Self>> {
        let s = Arc::new(RwLock::new(Self {
            providers_storage,
            error_logger,
            all_tasks: Vec::new(),
            changed_state_tasks: Vec::new(),
            tasks: SelectableList::default()
                .shortcut(Shortcut::new("Activate Tasks block", &['g', 't']))
                .show_count_in_title(false),
            projects_filter: Vec::new(),
            providers_filter: Vec::new(),
            commit_changes_shortcut: Shortcut::new("Commit changes", &['c', 'c']).global(),
            swap_completed_state_shortcut: Shortcut::new("Swap completed state of the task", &[' ']),
            in_progress_shortcut: Shortcut::new("Move the task in progress", &['p']),
            last_filter: Filter::default(),
        }));
        tokio::spawn({
            let s = s.clone();
            async move {
                let mut commit_changes_rx = s.read().await.commit_changes_shortcut.subscribe_to_accepted();
                let mut swap_completed_state_rx = s.read().await.swap_completed_state_shortcut.subscribe_to_accepted();
                let mut in_progress_rx = s.read().await.in_progress_shortcut.subscribe_to_accepted();
                loop {
                    tokio::select! {
                        _ = commit_changes_rx.recv() => {
                            let mut s = s.write().await;
                            if s.has_changes() {
                                s.commit_changes().await;
                            }
                        },
                        _ = swap_completed_state_rx.recv() => s.write().await.change_check_state(None).await,
                        _ = in_progress_rx.recv() => s.write().await.change_check_state(Some(task::State::InProgress)).await,
                    }
                }
            }
        });
        s
    }
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

    pub fn has_changes(&self) -> bool {
        !self.changed_state_tasks.is_empty()
    }

    pub async fn commit_changes(&mut self) {
        for p in self.providers_storage.write().await.iter_mut() {
            let name = p.name();
            let patches = self
                .changed_state_tasks
                .iter()
                .filter(|c| c.task.provider() == name)
                .map(|c| TaskPatch {
                    task: c.task.clone_boxed(),
                    state: Some(c.new_state.clone()),
                })
                .collect::<Vec<TaskPatch>>();

            if !patches.is_empty() {
                let errors = p.patch_tasks(&patches).await;

                let mut error_logger = self.error_logger.write().await;
                for e in &errors {
                    error_logger.add_error(
                        format!("Provider {name} returns error when changing the task: {}", e.error).as_str(),
                    );
                }

                for p in patches {
                    if !errors.iter().any(|pe| equal(p.task.as_ref(), pe.task.as_ref())) {
                        self.changed_state_tasks
                            .retain(|c| !equal(c.task.as_ref(), p.task.as_ref()));
                    }
                }

                p.reload().await;
            }
        }

        self.load_tasks(&self.last_filter.clone()).await;
    }

    async fn change_check_state(&mut self, state: Option<State>) {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return;
        }

        let t = selected.unwrap();
        let mut current_state = t.state();

        if let Some(i) = self
            .changed_state_tasks
            .iter()
            .position(|c| equal(c.task.as_ref(), t.as_ref()))
        {
            current_state = self.changed_state_tasks[i].new_state.clone();
            self.changed_state_tasks.remove(i);
            if state.as_ref().is_some_and(|s| *s == current_state) {
                return; // We undo the change
            }
        }
        let new_state = state.unwrap_or(match current_state {
            task::State::Completed => task::State::Uncompleted,
            task::State::Uncompleted | task::State::InProgress | task::State::Unknown(_) => task::State::Completed,
        });

        if new_state != t.state() {
            self.changed_state_tasks.push(ChangedState {
                task: t.clone_boxed(),
                new_state,
            });
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let changed = &self.changed_state_tasks;
        let mut title = format!("Tasks ({})", self.tasks.len());
        let tz = Local::now().timezone();

        if !changed.is_empty() {
            title = format!(
                "{title} (uncommitted count {}, use 'c'+'c' to commit them)",
                changed.len()
            )
        }

        self.tasks.render(
            title.as_str(),
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
                let (state, uncommitted) = match changed.iter().find(|c| equal(c.task.as_ref(), t.as_ref())) {
                    Some(c) => (c.new_state.clone(), true),
                    None => (t.state(), false),
                };
                let mut lines = vec![
                    Span::from(format!("[{state}] ")),
                    Span::styled(t.text(), Style::default().fg(fg_color)),
                    Span::from(" ("),
                    Span::styled(
                        format!("due: {}", task::datetime_to_str(t.due(), &tz)),
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

                if uncommitted {
                    lines.push(Span::from(" 📤"));
                }

                ListItem::from(Line::from(lines))
            },
            area,
            buf,
        );
    }

    fn remove_changed_tasks_that_are_not_exists_anymore(&mut self) {
        self.changed_state_tasks.retain(|c| {
            self.all_tasks
                .iter()
                .find(|t| equal(t.as_ref(), c.task.as_ref()))
                .is_some_and(|t| t.state() == c.task.state())
        });
    }

    pub async fn load_tasks(&mut self, f: &Filter) {
        self.last_filter = f.clone();

        let mut all_tasks: Vec<Box<dyn task::Task>> = Vec::new();

        let mut errors = Vec::new();
        for p in self.providers_storage.write().await.iter_mut() {
            let tasks = p.tasks(None, f).await;

            match tasks {
                Ok(t) => all_tasks.append(&mut t.iter().map(|t| t.clone_boxed()).collect::<Vec<Box<dyn TaskTrait>>>()),
                Err(err) => {
                    errors.push(format!("Load provider {} projects failure: {err}", p.name()));
                }
            }
        }

        self.error_logger
            .write()
            .await
            .add_errors(&errors.iter().map(|m| m.as_str()).collect::<Vec<&str>>());

        all_tasks.sort_by(|l, r| {
            due_group(l.as_ref())
                .cmp(&due_group(r.as_ref()))
                .then_with(|| r.priority().cmp(&l.priority()))
                .then_with(|| l.due().cmp(&r.due()))
        });

        self.all_tasks = all_tasks;
        self.remove_changed_tasks_that_are_not_exists_anymore();
        self.filter_tasks();
    }

    pub async fn reload(&mut self) {
        self.changed_state_tasks.clear();
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

#[async_trait]
impl MouseHandler for TasksWidget {
    async fn handle_mouse(&mut self, _ev: &MouseEvent) {}
}
