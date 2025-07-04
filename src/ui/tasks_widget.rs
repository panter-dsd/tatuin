// SPDX-License-Identifier: MIT

use super::{
    AppBlockWidget, dialogs::DialogTrait, dialogs::ListDialog, draw_helper::DrawHelper, header::Header,
    keyboard_handler::KeyboardHandler, mouse_handler::MouseHandler, shortcut::Shortcut, widgets::TaskRow,
    widgets::WidgetTrait,
};
use crate::{
    async_jobs::{AsyncJob, AsyncJobStorage},
    filter::Filter,
    patched_task::PatchedTask,
    project::Project as ProjectTrait,
    provider::Provider,
    task::{self, Priority, State, Task as TaskTrait, datetime_to_str, due_group},
    task_patch::{DuePatchItem, PatchError, TaskPatch},
    types::ArcRwLock,
};
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    text::Text,
    widgets::{Clear, ListState, Widget},
};
use std::{slice::IterMut, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use tracing::{Instrument, Level};

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
}
type ErrorLogger = ArcRwLock<dyn ErrorLoggerTrait>;

#[async_trait]
pub trait TaskInfoViewerTrait: Send + Sync {
    async fn set_task(&mut self, task: Option<Box<dyn TaskTrait>>);
}

type TaskInfoViewer = ArcRwLock<dyn TaskInfoViewerTrait>;

pub struct TasksWidget {
    providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>,
    error_logger: ErrorLogger,
    task_info_viewer: TaskInfoViewer,
    all_tasks: Vec<Box<dyn TaskTrait>>,
    changed_tasks: Vec<TaskPatch>,
    tasks: Vec<TaskRow>,
    providers_filter: Vec<String>,
    projects_filter: Vec<String>,
    draw_helper: Option<DrawHelper>,
    on_changes_broadcast: broadcast::Sender<()>,
    async_jobs_storage: ArcRwLock<AsyncJobStorage>,
    is_active: bool,
    state: ListState,

    activate_shortcut: Shortcut,
    commit_changes_shortcut: Shortcut,
    swap_completed_state_shortcut: Shortcut,
    in_progress_shortcut: Shortcut,
    change_due_shortcut: Shortcut,
    change_priority_shortcut: Shortcut,
    undo_changes_shortcut: Shortcut,

    last_filter: Filter,

    change_dalog: Option<Box<dyn DialogTrait>>,

    arc_self: Option<ArcRwLock<Self>>,
}

#[async_trait]
impl AppBlockWidget for TasksWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut> {
        vec![&mut self.activate_shortcut]
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
        self.is_active = is_active
    }

    async fn select_next(&mut self) {
        if self.state.selected().is_none_or(|i| i < self.tasks.len() - 1) {
            self.state.select_next();
            self.update_task_info_view().await;
        }
    }

    async fn select_previous(&mut self) {
        if self.state.selected().is_some_and(|i| i > 0) {
            self.state.select_previous();
            self.update_task_info_view().await;
        }
    }

    async fn select_first(&mut self) {
        if !self.tasks.is_empty() {
            self.state.select_first();
            self.update_task_info_view().await;
        }
    }

    async fn select_last(&mut self) {
        if !self.tasks.is_empty() {
            self.state.select(Some(self.tasks.len() - 1));
            self.update_task_info_view().await;
        }
    }
}

impl TasksWidget {
    pub async fn new(
        providers_storage: ArcRwLock<dyn ProvidersStorage<Provider>>,
        error_logger: ErrorLogger,
        task_info_viewer: TaskInfoViewer,
        async_jobs_storage: ArcRwLock<AsyncJobStorage>,
    ) -> ArcRwLock<Self> {
        let (tx, _) = broadcast::channel(1);

        let s = Arc::new(RwLock::new(Self {
            providers_storage,
            error_logger,
            task_info_viewer,
            all_tasks: Vec::new(),
            changed_tasks: Vec::new(),
            is_active: false,
            state: ListState::default(),
            activate_shortcut: Shortcut::new("Activate Tasks block", &['g', 't']),
            tasks: Vec::new(),
            projects_filter: Vec::new(),
            providers_filter: Vec::new(),
            draw_helper: None,
            on_changes_broadcast: tx,
            async_jobs_storage,
            commit_changes_shortcut: Shortcut::new("Commit changes", &['c', 'c']).global(),
            swap_completed_state_shortcut: Shortcut::new("Swap completed state of the task", &[' ']),
            in_progress_shortcut: Shortcut::new("Move the task in progress", &['p']),
            change_due_shortcut: Shortcut::new("Change due date of the task", &['c', 'd']),
            change_priority_shortcut: Shortcut::new("Change priority of the task", &['c', 'p']),
            undo_changes_shortcut: Shortcut::new("Undo changes", &['u']),
            last_filter: Filter::default(),
            change_dalog: None,
            arc_self: None,
        }));
        s.write().await.arc_self = Some(s.clone());
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
                        _ = in_progress_rx.recv() => {
                                let t = s.read().await.selected_task();
                                if t.is_some_and(|t| t.const_patch_policy().available_states.contains(&task::State::InProgress)) {
                                    s.write().await.change_check_state(Some(task::State::InProgress)).await
                                }
                            },
                        _ = change_due_rx.recv() => s.write().await.show_change_due_date_dialog().await,
                        _ = change_priority_rx.recv() => s.write().await.show_change_priority_dialog().await,
                        _ = undo_changes_rx.recv() => s.write().await.undo_changes().await,
                    }

                    s.write().await.update_task_info_view().await;
                    if let Some(dh) = &s.read().await.draw_helper {
                        dh.write().await.redraw();
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

    pub fn subscribe_on_changes(&self) -> broadcast::Receiver<()> {
        self.on_changes_broadcast.subscribe()
    }

    fn filter_tasks(&mut self) {
        self.tasks = self
            .all_tasks
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
            .map(|t| TaskRow::new(t.as_ref(), &self.changed_tasks))
            .collect();

        self.state = if self.all_tasks.is_empty() {
            ListState::default()
        } else {
            let selected_idx = self
                .state
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
    }

    pub fn tasks_projects(&self) -> Vec<Box<dyn ProjectTrait>> {
        let mut projects: Vec<Box<dyn ProjectTrait>> = Vec::new();

        for t in self.tasks.iter() {
            if let Some(tp) = t.task().project() {
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
        if self.tasks.is_empty() {
            return None;
        }

        let t = self
            .state
            .selected()
            .map(|i| self.tasks[std::cmp::min(i, self.tasks.len() - 1)].task())?;
        let p = self.changed_tasks.iter().find(|p| p.is_task(t));
        Some(Box::new(PatchedTask::new(t.clone_boxed(), p.cloned())))
    }

    pub fn has_changes(&self) -> bool {
        !self.changed_tasks.is_empty()
    }

    pub async fn commit_changes(&mut self) {
        for p in self.providers_storage.write().await.iter_mut() {
            let name = &p.name;
            let patches = self
                .changed_tasks
                .iter()
                .filter(|c| &c.task.provider() == name)
                .map(|c| TaskPatch {
                    task: c.task.clone_boxed(),
                    state: c.state.clone(),
                    due: c.due.clone(),
                    priority: c.priority.clone(),
                })
                .collect::<Vec<TaskPatch>>();

            if patches.is_empty() {
                continue;
            }

            let errors = p.provider.write().await.patch_tasks(&patches).await;
            self.process_patch_errors(name, &errors).await;

            self.changed_tasks.retain(|c| {
                let patched = patches.iter().any(|tp| c.is_task(tp.task.as_ref()))
                    && !errors.iter().any(|pe| pe.is_task(c.task.as_ref()));
                !patched
            });

            p.provider.write().await.reload().await;
        }

        self.load_tasks(&self.last_filter.clone()).await;
    }

    async fn process_patch_errors(&self, provider_name: &str, errors: &[PatchError]) {
        let mut error_logger = self.error_logger.write().await;
        for e in errors {
            error_logger.add_error(
                format!(
                    "Provider {provider_name} returns error when changing the task: {}",
                    e.error
                )
                .as_str(),
            );
        }
    }

    async fn render_change_due_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        if let Some(d) = &mut self.change_dalog {
            let size = d.size();
            let idx = self.state.selected().unwrap_or(0) as u16;

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
        let s = self.arc_self.as_ref().unwrap().clone();

        tracing::event!(name: "load_tasks", Level::INFO, filter = ?&f, "Load tasks");

        for p in self.providers_storage.write().await.iter_mut() {
            tokio::spawn({
                let name = p.name.clone();
                let s = s.clone();
                let p = p.provider.clone();
                let f = f.clone();
                let async_jobs = self.async_jobs_storage.clone();

                let span = tracing::span!(Level::INFO, "load_provider_tasks", name = name, "Load provider's tasks");
                async move {
                    let _job = AsyncJob::new(format!("Load tasks from provider {name}").as_str(), async_jobs).await;
                    let tasks = p.write().await.tasks(None, &f).await;

                    let mut s = s.write().await;
                    s.all_tasks.retain(|t| t.provider() != name);

                    match tasks {
                        Ok(t) => {
                            s.all_tasks
                                .append(&mut t.iter().map(|t| t.clone_boxed()).collect::<Vec<Box<dyn TaskTrait>>>());
                            s.all_tasks.sort_by(|l, r| {
                                due_group(l.as_ref())
                                    .cmp(&due_group(r.as_ref()))
                                    .then_with(|| r.priority().cmp(&l.priority()))
                                    .then_with(|| l.due().cmp(&r.due()))
                                    .then_with(|| l.text().cmp(&r.text()))
                            });

                            s.remove_changed_tasks_that_are_not_exists_anymore();
                            s.filter_tasks();
                            s.update_task_info_view().await;
                            let _ = s.on_changes_broadcast.send(());
                        }
                        Err(err) => {
                            s.error_logger
                                .write()
                                .await
                                .add_error(format!("Load provider {name} projects failure: {err}").as_str());
                        }
                    }
                }
                .instrument(span)
            });
        }
    }

    pub async fn reload(&mut self) {
        self.changed_tasks.clear();
    }

    async fn show_change_due_date_dialog(&mut self) {
        let t = self.selected_task();
        if t.is_none() {
            return;
        }

        let t = t.unwrap();
        let available_due_items = t.patch_policy().available_due_items;
        if !available_due_items.is_empty() {
            let d = ListDialog::new(
                &available_due_items,
                datetime_to_str(t.due(), &Local::now().timezone()).as_str(),
            );
            self.change_dalog = Some(Box::new(d));
        }
    }

    async fn show_change_priority_dialog(&mut self) {
        let t = self.selected_task();
        if t.is_none() {
            return;
        }

        let t = t.unwrap();
        let available_priorities = t.patch_policy().available_priorities;
        if !available_priorities.is_empty() {
            let d = ListDialog::new(&available_priorities, t.priority().to_string().as_str());
            self.change_dalog = Some(Box::new(d));
        }
    }

    async fn change_check_state(&mut self, state: Option<State>) {
        let span = tracing::span!(Level::TRACE,
            "tasks_widget",
            state=?&state,
            selected=tracing::field::Empty,
            current_state=tracing::field::Empty,
            existed_patch=tracing::field::Empty,
            new_state=tracing::field::Empty,
            "Change check state");
        let _enter = span.enter();

        let selected = self.state.selected();
        if selected.is_none() {
            return;
        }

        span.record("selected", selected);

        let patched_task = self.selected_task().unwrap();
        let t = self.tasks[selected.unwrap()].task();

        let mut current_state = t.state();
        span.record("current_state", current_state.to_string());

        if let Some(p) = self.changed_tasks.iter_mut().find(|c| c.is_task(t)) {
            span.record("existed_patch", p.to_string());
            if let Some(s) = &p.state {
                current_state = s.clone();
                span.record("current_state", current_state.to_string());
                p.state = None;
                if p.is_empty() {
                    self.changed_tasks.retain(|c| !c.is_task(t));
                }
            }
        }

        let new_state = state.unwrap_or(match current_state {
            task::State::Completed => task::State::Uncompleted,
            task::State::Uncompleted | task::State::InProgress | task::State::Unknown(_) => task::State::Completed,
        });
        span.record("new_state", new_state.to_string());

        if patched_task.patch_policy().available_states.contains(&new_state) && (new_state != t.state()) {
            match self.changed_tasks.iter_mut().find(|p| p.is_task(t)) {
                Some(p) => p.state = Some(new_state),
                None => self.changed_tasks.push(TaskPatch {
                    task: t.clone_boxed(),
                    state: Some(new_state),
                    due: None,
                    priority: None,
                }),
            }
        }

        self.recreate_current_task_row().await;
    }

    async fn change_due_date(&mut self, due: &DuePatchItem) {
        let selected = self.state.selected();
        if selected.is_none() {
            return;
        }

        let t = self.tasks[selected.unwrap()].task();
        match self.changed_tasks.iter_mut().find(|p| p.is_task(t)) {
            Some(p) => p.due = Some(due.clone()),
            None => self.changed_tasks.push(TaskPatch {
                task: t.clone_boxed(),
                state: None,
                due: Some(due.clone()),
                priority: None,
            }),
        }
        self.recreate_current_task_row().await;
    }

    async fn change_priority(&mut self, priority: &Priority) {
        let selected = self.state.selected();
        if selected.is_none() {
            return;
        }

        let t = self.tasks[selected.unwrap()].task();
        match self.changed_tasks.iter_mut().find(|p| p.is_task(t)) {
            Some(p) => {
                p.priority = if *priority == t.priority() {
                    None
                } else {
                    Some(priority.clone())
                };
                if p.is_empty() {
                    self.changed_tasks.retain(|c| !c.is_task(t));
                }
            }
            None => self.changed_tasks.push(TaskPatch {
                task: t.clone_boxed(),
                state: None,
                due: None,
                priority: Some(priority.clone()),
            }),
        }
        self.recreate_current_task_row().await;
    }

    async fn undo_changes(&mut self) {
        let selected = self.state.selected();
        if selected.is_none() {
            return;
        }

        let t = self.tasks[selected.unwrap()].task();
        self.changed_tasks.retain(|p| !p.is_task(t));
        self.recreate_current_task_row().await;
    }

    async fn recreate_current_task_row(&mut self) {
        let idx = self.state.selected().unwrap();
        self.tasks[idx] = TaskRow::new(self.tasks[idx].task(), &self.changed_tasks);
    }

    async fn update_task_info_view(&mut self) {
        self.task_info_viewer.write().await.set_task(self.selected_task()).await;
    }
}

#[async_trait]
impl MouseHandler for TasksWidget {
    async fn handle_mouse(&mut self, ev: &MouseEvent) {
        for w in self.tasks.iter_mut() {
            w.handle_mouse(ev).await;
        }
    }
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
                if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<ListDialog<DuePatchItem>>() {
                    new_due = d.selected().clone();
                }
                if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<ListDialog<Priority>>() {
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

#[async_trait]
impl WidgetTrait for TasksWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let changed = &self.changed_tasks;
        let mut title = format!("Tasks ({})", self.tasks.len());

        if !changed.is_empty() {
            title.push_str(format!(" (uncommitted count {}, use 'c'+'c' to commit them)", changed.len()).as_str());
        }

        let h = Header::new(title.as_str(), self.is_active, Some(&self.activate_shortcut));
        h.block().render(area, buf);

        let mut y = area.y + 1;

        let selected = self.state.selected();

        for (i, w) in self.tasks.iter_mut().enumerate() {
            let is_row_selected = selected.is_some_and(|idx| idx == i);
            w.set_selected(is_row_selected);
            if is_row_selected {
                Text::from(">").render(
                    Rect {
                        x: area.x,
                        y,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            }
            w.set_pos(Position::new(area.x + 1, y));
            w.render(area, buf).await;
            y += 1;
        }

        if self.change_dalog.is_some() {
            self.render_change_due_dialog(area, buf).await;
        }
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.draw_helper = Some(dh)
    }

    fn size(&self) -> Size {
        Size::default()
    }
}
