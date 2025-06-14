// SPDX-License-Identifier: MIT

use super::AppBlockWidget;
use super::keyboard_handler::KeyboardHandler;
use super::list_dialog;
use super::mouse_handler::MouseHandler;
use crate::filter::Filter;
use crate::patched_task::PatchedTask;
use crate::project::Project as ProjectTrait;
use crate::provider::Provider as ProviderTrait;
use crate::state::StatefulObject;
use crate::task::{self, Priority, datetime_to_str};
use crate::task::{State, Task as TaskTrait, due_group};
use crate::task_patch::{DuePatchItem, TaskPatch};
use crate::ui::selectable_list::SelectableList;
use crate::ui::{dialog, style};
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::KeyEvent;
use crossterm::event::MouseEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, ListItem, ListState, Widget};
use std::cmp::Ordering;
use std::slice::IterMut;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

use super::shortcut::Shortcut;

impl std::fmt::Display for DuePatchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuePatchItem::Today => write!(f, "Today"),
            DuePatchItem::Tomorrow => write!(f, "Tomorrow"),
            DuePatchItem::ThisWeekend => write!(f, "This weekend"),
            DuePatchItem::NextWeek => write!(f, "Next week (Monday)"),
            DuePatchItem::NoDate => write!(f, "No date"),
        }
    }
}

pub trait ProvidersStorage<T>: Send + Sync {
    fn iter_mut(&mut self) -> IterMut<'_, T>;
}

pub trait ErrorLoggerTrait: Send + Sync {
    fn add_error(&mut self, message: &str);
    fn add_errors(&mut self, messages: &[&str]);
}
type ErrorLogger = Arc<RwLock<dyn ErrorLoggerTrait>>;

pub trait TaskInfoViewerTrait: Send + Sync {
    fn set_task(&mut self, task: Option<Box<dyn TaskTrait>>);
}
type TaskInfoViewer = Arc<RwLock<dyn TaskInfoViewerTrait>>;

pub struct TasksWidget {
    providers_storage: Arc<RwLock<dyn ProvidersStorage<Box<dyn ProviderTrait>>>>,
    error_logger: ErrorLogger,
    task_info_viewer: TaskInfoViewer,
    all_tasks: Vec<Box<dyn TaskTrait>>,
    changed_tasks: Vec<TaskPatch>,
    tasks: SelectableList<Box<dyn TaskTrait>>,
    providers_filter: Vec<String>,
    projects_filter: Vec<String>,
    redraw_tx: Option<mpsc::UnboundedSender<()>>,

    commit_changes_shortcut: Shortcut,
    swap_completed_state_shortcut: Shortcut,
    in_progress_shortcut: Shortcut,
    change_due_shortcut: Shortcut,
    change_priority_shortcut: Shortcut,
    undo_changes_shortcut: Shortcut,

    last_filter: Filter,

    change_dalog: Option<Box<dyn dialog::DialogTrait>>,
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
            &mut self.change_due_shortcut,
            &mut self.change_priority_shortcut,
            &mut self.undo_changes_shortcut,
        ]
    }

    fn set_active(&mut self, is_active: bool) {
        self.tasks.set_active(is_active);
    }

    async fn select_next(&mut self) {
        self.tasks.select_next().await;
        self.update_task_info_view().await;
    }

    async fn select_previous(&mut self) {
        self.tasks.select_previous().await;
        self.update_task_info_view().await;
    }

    async fn select_first(&mut self) {
        self.tasks.select_first().await;
        self.update_task_info_view().await;
    }

    async fn select_last(&mut self) {
        self.tasks.select_last().await;
        self.update_task_info_view().await;
    }

    fn set_redraw_tx(&mut self, tx: mpsc::UnboundedSender<()>) {
        self.redraw_tx = Some(tx)
    }
}

impl TasksWidget {
    pub fn new(
        providers_storage: Arc<RwLock<dyn ProvidersStorage<Box<dyn ProviderTrait>>>>,
        error_logger: ErrorLogger,
        task_info_viewer: TaskInfoViewer,
    ) -> Arc<RwLock<Self>> {
        let s = Arc::new(RwLock::new(Self {
            providers_storage,
            error_logger,
            task_info_viewer,
            all_tasks: Vec::new(),
            changed_tasks: Vec::new(),
            tasks: SelectableList::default()
                .shortcut(Shortcut::new("Activate Tasks block", &['g', 't']))
                .show_count_in_title(false),
            projects_filter: Vec::new(),
            providers_filter: Vec::new(),
            redraw_tx: None,
            commit_changes_shortcut: Shortcut::new("Commit changes", &['c', 'c']).global(),
            swap_completed_state_shortcut: Shortcut::new("Swap completed state of the task", &[' ']),
            in_progress_shortcut: Shortcut::new("Move the task in progress", &['p']),
            change_due_shortcut: Shortcut::new("Change due date of the task", &['c', 'd']),
            change_priority_shortcut: Shortcut::new("Change priority of the task", &['c', 'p']),
            undo_changes_shortcut: Shortcut::new("Undo changes", &['u']),
            last_filter: Filter::default(),
            change_dalog: None,
        }));
        tokio::spawn({
            let s = s.clone();
            async move {
                let mut commit_changes_rx = s.read().await.commit_changes_shortcut.subscribe_to_accepted();
                let mut swap_completed_state_rx = s.read().await.swap_completed_state_shortcut.subscribe_to_accepted();
                let mut in_progress_rx = s.read().await.in_progress_shortcut.subscribe_to_accepted();
                let mut change_due_rx = s.read().await.change_due_shortcut.subscribe_to_accepted();
                let mut change_priority_rx = s.read().await.change_priority_shortcut.subscribe_to_accepted();
                let mut undo_changes_rx = s.read().await.undo_changes_shortcut.subscribe_to_accepted();
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
                        _ = change_due_rx.recv() => s.write().await.show_change_due_date_dialog().await,
                        _ = change_priority_rx.recv() => s.write().await.show_change_priority_dialog().await,
                        _ = undo_changes_rx.recv() => s.write().await.undo_changes().await,
                    }

                    s.write().await.update_task_info_view().await;
                    if let Some(tx) = &s.read().await.redraw_tx {
                        let _ = tx.send(());
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
        let t = self.tasks.selected().map(|t| t.clone_boxed());
        if t.is_none() {
            return t;
        }
        let t = t.unwrap();
        let p = self.changed_tasks.iter().find(|p| p.is_task(t.as_ref())).cloned();
        Some(Box::new(PatchedTask::new(t, p)))
    }

    pub fn has_changes(&self) -> bool {
        !self.changed_tasks.is_empty()
    }

    pub async fn commit_changes(&mut self) {
        for p in self.providers_storage.write().await.iter_mut() {
            let name = p.name();
            let patches = self
                .changed_tasks
                .iter()
                .filter(|c| c.task.provider() == name)
                .map(|c| TaskPatch {
                    task: c.task.clone_boxed(),
                    state: c.state.clone(),
                    due: c.due.clone(),
                    priority: c.priority.clone(),
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
                    if !errors.iter().any(|pe| pe.is_task(p.task.as_ref())) {
                        self.changed_tasks.retain(|c| !c.is_task(p.task.as_ref()));
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

        if let Some(p) = self.changed_tasks.iter_mut().find(|c| c.is_task(t.as_ref())) {
            if let Some(s) = &p.state {
                current_state = s.clone();
                p.state = None;
                if state.as_ref().is_some_and(|s| *s == current_state) {
                    if p.is_empty() {
                        self.changed_tasks.retain(|c| !c.is_task(t.as_ref()));
                    }
                    return; // We undo the change
                }
            }
        }
        let new_state = state.unwrap_or(match current_state {
            task::State::Completed => task::State::Uncompleted,
            task::State::Uncompleted | task::State::InProgress | task::State::Unknown(_) => task::State::Completed,
        });

        if new_state != t.state() {
            match self.changed_tasks.iter_mut().find(|p| p.is_task(t.as_ref())) {
                Some(p) => p.state = Some(new_state),
                None => self.changed_tasks.push(TaskPatch {
                    task: t.clone_boxed(),
                    state: Some(new_state),
                    due: None,
                    priority: None,
                }),
            }
        }
    }

    pub async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let changed = &self.changed_tasks;
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
                let mut state = t.state();
                let mut due = task::datetime_to_str(t.due(), &tz);
                let mut priority = t.priority();
                let mut uncommitted = false;
                if let Some(patch) = changed.iter().find(|c| c.is_task(t.as_ref())) {
                    uncommitted = !patch.is_empty();
                    if let Some(s) = &patch.state {
                        state = s.clone();
                    }
                    if let Some(d) = &patch.due {
                        due = d.to_string();
                    }
                    if let Some(p) = &patch.priority {
                        priority = p.clone();
                    }
                }

                let mut lines = vec![
                    Span::from(format!("[{state}] ")),
                    Span::styled(t.text(), Style::default().fg(fg_color)),
                    Span::from(" ("),
                    Span::styled(format!("due: {due}"), Style::default().fg(Color::Blue)),
                    Span::from(") ("),
                    Span::styled(format!("Priority: {priority}"), style::priority_color(&priority)),
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

        if self.change_dalog.is_some() {
            self.render_change_due_dialog(area, buf).await;
        }
    }

    async fn render_change_due_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        if let Some(d) = &mut self.change_dalog {
            let size = d.size();
            let idx = self.tasks.selected_index().unwrap_or(0) as u16;

            let mut y = area.y + 1 /*title*/ + idx+1/*right below the item*/;
            if area.height - y < size.height {
                y = area.y + area.height - size.height;
            }

            let mut area = area;
            area.y = y;
            area.height = size.height;
            area.x += 1; //TODO: constant
            area.width = std::cmp::min(size.width, area.width - area.x);

            Clear {}.render(area, buf);
            d.render(area, buf).await;
        }
    }

    fn remove_changed_tasks_that_are_not_exists_anymore(&mut self) {
        self.changed_tasks.retain(|c| {
            self.all_tasks
                .iter()
                .find(|t| c.is_task(t.as_ref()))
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
        self.update_task_info_view().await;
    }

    pub async fn reload(&mut self) {
        self.changed_tasks.clear();
    }

    async fn show_change_due_date_dialog(&mut self) {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return;
        }

        let t = selected.unwrap();
        let d = list_dialog::Dialog::new(
            &[
                DuePatchItem::Today,
                DuePatchItem::Tomorrow,
                DuePatchItem::ThisWeekend,
                DuePatchItem::NextWeek,
                DuePatchItem::NoDate,
            ],
            datetime_to_str(t.due(), &Local::now().timezone()).as_str(),
        );
        self.change_dalog = Some(Box::new(d));
    }

    async fn show_change_priority_dialog(&mut self) {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return;
        }

        let t = selected.unwrap();
        let d = list_dialog::Dialog::new(
            &[
                Priority::Normal,
                Priority::Lowest,
                Priority::Low,
                Priority::Medium,
                Priority::High,
                Priority::Highest,
            ],
            t.priority().to_string().as_str(),
        );
        self.change_dalog = Some(Box::new(d));
    }

    async fn change_due_date(&mut self, due: &DuePatchItem) {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return;
        }

        let t = selected.unwrap();
        match self.changed_tasks.iter_mut().find(|p| p.is_task(t.as_ref())) {
            Some(p) => p.due = Some(due.clone()),
            None => self.changed_tasks.push(TaskPatch {
                task: t.clone_boxed(),
                state: None,
                due: Some(due.clone()),
                priority: None,
            }),
        }
    }

    async fn change_priority(&mut self, priority: &Priority) {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return;
        }

        let t = selected.unwrap();
        match self.changed_tasks.iter_mut().find(|p| p.is_task(t.as_ref())) {
            Some(p) => p.priority = Some(priority.clone()),
            None => self.changed_tasks.push(TaskPatch {
                task: t.clone_boxed(),
                state: None,
                due: None,
                priority: Some(priority.clone()),
            }),
        }
    }

    async fn undo_changes(&mut self) {
        let selected = self.tasks.selected();
        if selected.is_none() {
            return;
        }

        let t = selected.unwrap();
        self.changed_tasks.retain(|p| !p.is_task(t.as_ref()));
    }

    async fn update_task_info_view(&mut self) {
        self.task_info_viewer.write().await.set_task(self.selected_task());
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

#[async_trait]
impl KeyboardHandler for TasksWidget {
    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        let mut handled = false;

        let mut need_to_update_view = false;
        let mut new_due = None;
        let mut new_priority = None;

        if let Some(d) = &mut self.change_dalog {
            need_to_update_view = true;
            handled = d.handle_key(key).await;
            if handled && d.should_be_closed() {
                if let Some(d) = d.as_any().downcast_ref::<list_dialog::Dialog<DuePatchItem>>() {
                    new_due = d.selected().clone();
                }
                if let Some(d) = d.as_any().downcast_ref::<list_dialog::Dialog<Priority>>() {
                    new_priority = d.selected().clone();
                }
                self.change_dalog = None;
            }
        };
        if let Some(due) = new_due {
            self.change_due_date(&due).await;
        }
        if let Some(p) = new_priority {
            self.change_priority(&p).await;
        }

        if need_to_update_view {
            self.update_task_info_view().await;
        }

        handled
    }
}
