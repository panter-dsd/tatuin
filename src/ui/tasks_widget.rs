// SPDX-License-Identifier: MIT

use super::{
    AppBlockWidget,
    dialogs::{CreateUpdateTaskDialog, DialogTrait, ListDialog},
    draw_helper::{DrawHelper, global_dialog_area},
    header::Header,
    keyboard_handler::KeyboardHandler,
    mouse_handler::MouseHandler,
    shortcut::Shortcut,
    style::default_style,
    widgets::{DateEditor, TaskRow, WidgetState, WidgetStateTrait, WidgetTrait},
};
use crate::{
    async_jobs::{AsyncJob, AsyncJobStorage},
    filter::Filter,
    patched_task::PatchedTask,
    project::Project as ProjectTrait,
    provider::Provider,
    style,
    task::{self, DateTimeUtc, Priority, State, Task as TaskTrait, datetime_to_str, due_group},
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
    widgets::{Block, Clear, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};
use std::{any::Any, slice::Iter, slice::IterMut, sync::Arc};
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
            DuePatchItem::Custom(d) => {
                if d == &DateTimeUtc::default() {
                    write!(f, "Custom")
                } else {
                    let tz = Local::now().timezone();
                    write!(f, "Custom ({})", datetime_to_str(Some(*d), &tz))
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct Patch {
    provider_name: Option<String>,
    project_id: Option<String>,
    task_patch: Option<TaskPatch>,
}

impl Patch {
    fn is_valid(&self) -> bool {
        self.provider_name.is_some() && self.project_id.is_some() && self.task_patch.is_some()
    }
}

pub trait ProvidersStorage: Send + Sync {
    fn iter_mut<'a>(&'a mut self) -> IterMut<'a, Provider>;
    fn iter<'a>(&'a self) -> Iter<'a, Provider>;
    fn provider(&self, name: &str) -> Provider;
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
    providers_storage: ArcRwLock<dyn ProvidersStorage>,
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
    list_state: ListState,
    widget_state: WidgetState,

    activate_shortcut: Shortcut,
    commit_changes_shortcut: Shortcut,
    swap_completed_state_shortcut: Shortcut,
    in_progress_shortcut: Shortcut,
    change_due_shortcut: Shortcut,
    change_priority_shortcut: Shortcut,
    undo_changes_shortcut: Shortcut,
    add_task_shortcut: Shortcut,
    edit_task_shortcut: Shortcut,
    open_task_link_shortcut: Shortcut,

    last_filter: Filter,

    dialog: Option<Box<dyn DialogTrait>>,
    is_global_dialog: bool,

    arc_self: Option<ArcRwLock<Self>>,
}
crate::impl_widget_state_trait!(TasksWidget);

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
            &mut self.add_task_shortcut,
            &mut self.edit_task_shortcut,
            &mut self.open_task_link_shortcut,
        ]
    }

    async fn select_next(&mut self) {
        if self.list_state.selected().is_none_or(|i| i < self.tasks.len() - 1) {
            self.list_state.select_next();
            self.update_task_info_view().await;
        }
    }

    async fn select_previous(&mut self) {
        if self.list_state.selected().is_some_and(|i| i > 0) {
            self.list_state.select_previous();
            self.update_task_info_view().await;
        }
    }

    async fn select_first(&mut self) {
        if !self.tasks.is_empty() {
            self.list_state.select_first();
            self.update_task_info_view().await;
        }
    }

    async fn select_last(&mut self) {
        if !self.tasks.is_empty() {
            self.list_state.select(Some(self.tasks.len() - 1));
            self.update_task_info_view().await;
        }
    }
}

impl TasksWidget {
    pub async fn new(
        providers_storage: ArcRwLock<dyn ProvidersStorage>,
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
            list_state: ListState::default(),
            widget_state: WidgetState::default(),
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
            add_task_shortcut: Shortcut::new("Create a task", &['a']),
            edit_task_shortcut: Shortcut::new("Edit the task", &['e']),
            open_task_link_shortcut: Shortcut::new("Open the task's link", &['o']),
            last_filter: Filter::default(),
            dialog: None,
            is_global_dialog: true,
            arc_self: None,
        }));
        s.write().await.arc_self = Some(s.clone());
        tokio::spawn({
            let s = s.clone();
            async move {
                let s_guard = s.read().await;
                let mut commit_changes_rx = s_guard.commit_changes_shortcut.subscribe_to_accepted();
                let mut swap_completed_state_rx = s_guard.swap_completed_state_shortcut.subscribe_to_accepted();
                let mut in_progress_rx = s_guard.in_progress_shortcut.subscribe_to_accepted();
                let mut change_due_rx = s_guard.change_due_shortcut.subscribe_to_accepted();
                let mut change_priority_rx = s_guard.change_priority_shortcut.subscribe_to_accepted();
                let mut undo_changes_rx = s_guard.undo_changes_shortcut.subscribe_to_accepted();
                let mut add_task_rx = s_guard.add_task_shortcut.subscribe_to_accepted();
                let mut edit_task_rx = s_guard.edit_task_shortcut.subscribe_to_accepted();
                let mut open_task_link_rx = s_guard.open_task_link_shortcut.subscribe_to_accepted();
                drop(s_guard);
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
                        _ = add_task_rx.recv() => s.write().await.show_add_task_dialog(None).await,
                        _ = edit_task_rx.recv() => {
                            let mut s = s.write().await;
                            if let Some(t) = s.selected_task()
                                && t.patch_policy().is_editable {
                                s.show_add_task_dialog(Some(t)).await;
                            }
                        },
                        _ = open_task_link_rx.recv() => {
                            if let Some(t) = s.read().await.selected_task()
                                && !t.url().is_empty()
                                && let Err(e) = crate::utils::open_url(t.url().as_str()){
                                s.write().await.error_logger.write().await.add_error(e.to_string().as_str());
                            }
                        }
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
                if let Some(tp) = t.project()
                    && !self.projects_filter.is_empty()
                    && !self.projects_filter.contains(&tp.name())
                {
                    result = false;
                }
                result
            })
            .map(|t| TaskRow::new(t.as_ref(), &self.changed_tasks))
            .collect();

        self.list_state = if self.all_tasks.is_empty() {
            ListState::default()
        } else {
            let selected_idx = self
                .list_state
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
            .list_state
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
                .filter(|c| c.task.as_ref().is_some_and(|t| &t.provider() == name))
                .cloned()
                .collect::<Vec<TaskPatch>>();

            if patches.is_empty() {
                continue;
            }

            let errors = p.provider.write().await.patch_tasks(&patches).await;
            self.process_patch_errors(name, &errors).await;

            self.changed_tasks.retain(|c| {
                let patched = patches
                    .iter()
                    .any(|tp| tp.task.as_ref().is_some_and(|t| c.is_task(t.as_ref())))
                    && !errors
                        .iter()
                        .any(|pe| c.task.as_ref().is_some_and(|t| pe.is_task(t.as_ref())));
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

    fn inline_dialog_area(&self, size: Size, area: Rect) -> Rect {
        let idx = self.list_state.selected().unwrap_or(0) as u16;

        let mut y = area.y + 1 /*title*/ + idx+1/*right below the item*/;
        if area.height - y < size.height {
            y = area.y + area.height - size.height;
        }

        Rect {
            x: area.x + 1, //TODO: constant
            y,
            width: std::cmp::min(size.width, area.width - area.x),
            height: size.height,
        }
    }

    async fn render_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        if let Some(dh) = &self.draw_helper {
            let screen_size = dh.read().await.screen_size();
            let d = self.dialog.as_mut().unwrap();
            let min_size = d.min_size();
            let size = Size::new(min_size.width.max(screen_size.width / 2), min_size.height);
            d.set_size(size);
        }
        let size = self.dialog.as_ref().unwrap().size();
        let area = if self.is_global_dialog {
            global_dialog_area(size, *buf.area())
        } else {
            self.inline_dialog_area(size, area)
        };

        Clear {}.render(area, buf);

        let d = self.dialog.as_mut().unwrap();
        d.render(area, buf).await;
    }

    fn remove_changed_tasks_that_are_not_exists_anymore(&mut self) {
        self.changed_tasks.retain(|c| {
            self.all_tasks
                .iter()
                .find(|t| c.is_task(t.as_ref()))
                .is_some_and(|t| c.task.as_ref().is_some_and(|task| t.state() == task.state()))
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
                                    .then_with(|| project_name(l.as_ref()).cmp(&project_name(r.as_ref())))
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
            let mut d = ListDialog::new(
                &available_due_items,
                datetime_to_str(t.due(), &Local::now().timezone()).as_str(),
            );
            d.add_custom_widget(
                DuePatchItem::Custom(DateTimeUtc::default()),
                Arc::new(DateEditor::new(t.due())),
            );
            self.dialog = Some(Box::new(d));
            self.is_global_dialog = false;
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
            self.dialog = Some(Box::new(d));
            self.is_global_dialog = false;
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

        let selected = self.list_state.selected();
        if selected.is_none() {
            return;
        }

        span.record("selected", selected);

        let patched_task = self.selected_task();
        if patched_task.is_none() {
            return;
        }

        let patched_task = patched_task.unwrap();
        let t = self.tasks[selected.unwrap()].task();

        let mut current_state = t.state();
        span.record("current_state", current_state.to_string());

        if let Some(p) = self.changed_tasks.iter_mut().find(|c| c.is_task(t)) {
            span.record("existed_patch", p.to_string());
            if let Some(s) = &p.state {
                current_state = *s;
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
                    task: Some(t.clone_boxed()),
                    name: None,
                    description: None,
                    due: None,
                    priority: None,
                    state: Some(new_state),
                }),
            }
        }

        self.recreate_current_task_row().await;
    }

    async fn change_due_date(&mut self, due: &DuePatchItem) {
        let selected = self.list_state.selected();
        if selected.is_none() {
            return;
        }

        let t = self.tasks[selected.unwrap()].task();
        match self.changed_tasks.iter_mut().find(|p| p.is_task(t)) {
            Some(p) => p.due = Some(*due),
            None => self.changed_tasks.push(TaskPatch {
                task: Some(t.clone_boxed()),
                name: None,
                description: None,
                due: Some(*due),
                priority: None,
                state: None,
            }),
        }
        self.recreate_current_task_row().await;
    }

    async fn change_priority(&mut self, priority: &Priority) {
        let selected = self.list_state.selected();
        if selected.is_none() {
            return;
        }

        let t = self.tasks[selected.unwrap()].task();
        match self.changed_tasks.iter_mut().find(|p| p.is_task(t)) {
            Some(p) => {
                p.priority = if *priority == t.priority() {
                    None
                } else {
                    Some(*priority)
                };
                if p.is_empty() {
                    self.changed_tasks.retain(|c| !c.is_task(t));
                }
            }
            None => self.changed_tasks.push(TaskPatch {
                task: Some(t.clone_boxed()),
                name: None,
                description: None,
                due: None,
                priority: Some(*priority),
                state: None,
            }),
        }
        self.recreate_current_task_row().await;
    }

    async fn undo_changes(&mut self) {
        let selected = self.list_state.selected();
        if selected.is_none() {
            return;
        }

        let t = self.tasks[selected.unwrap()].task();
        self.changed_tasks.retain(|p| !p.is_task(t));
        self.recreate_current_task_row().await;
    }

    async fn recreate_current_task_row(&mut self) {
        let idx = self.list_state.selected().unwrap();
        self.tasks[idx] = TaskRow::new(self.tasks[idx].task(), &self.changed_tasks);
    }

    async fn update_task_info_view(&mut self) {
        self.task_info_viewer.write().await.set_task(self.selected_task()).await;
    }

    async fn show_add_task_dialog(&mut self, task: Option<Box<dyn TaskTrait>>) {
        let title = if task.is_some() {
            "Update the task"
        } else {
            "Create a task"
        };

        let mut d = CreateUpdateTaskDialog::new(title, self.providers_storage.clone()).await;
        if let Some(t) = task {
            d.set_task(t.as_ref()).await;
        }
        if let Some(dh) = &self.draw_helper {
            d.set_draw_helper(dh.clone());
        }
        self.dialog = Some(Box::new(d));
        self.is_global_dialog = true;
    }

    #[tracing::instrument(level = "info", target = "tasks_widget")]
    async fn create_or_update_task(&mut self, patch: &Patch) {
        let provider = self
            .providers_storage
            .read()
            .await
            .provider(patch.provider_name.as_ref().unwrap());
        let project_id = patch.project_id.as_ref().unwrap();
        let tp = patch.task_patch.as_ref().unwrap();

        if tp.task.is_some() {
            let patched_task = tp
                .task
                .as_ref()
                .unwrap()
                .as_any()
                .downcast_ref::<PatchedTask>()
                .expect("Wrong logic! PatchedTask was expected.");

            let task = patched_task.original_task();

            match self.changed_tasks.iter_mut().find(|p| p.is_task(task.as_ref())) {
                Some(p) => {
                    replace_if(&mut p.name, &tp.name);
                    replace_if(&mut p.description, &tp.description);
                    replace_if(&mut p.due, &tp.due);
                    replace_if(&mut p.priority, &tp.priority);
                    replace_if(&mut p.state, &tp.state);
                }
                None => {
                    let mut tp = tp.clone();
                    tp.task = Some(task);
                    self.changed_tasks.push(tp);
                }
            }
            self.recreate_current_task_row().await;
        } else {
            let mut provider = provider.provider.write().await;
            match provider.create_task(project_id, tp).await {
                Ok(()) => {
                    provider.reload().await;
                }
                Err(e) => {
                    tracing::error!(error=?e, "Create a task");
                    self.error_logger.write().await.add_error(e.to_string().as_str());
                }
            };
            self.load_tasks(&self.last_filter.clone()).await;
        }
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
        let mut patch = Patch::default();
        let mut add_another_one_task = false;

        if let Some(d) = &mut self.dialog {
            need_to_update_view = true;
            handled = d.handle_key(key).await;
            if handled && d.should_be_closed() {
                if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<ListDialog<DuePatchItem>>() {
                    new_due = d.selected().map(|p| match p {
                        DuePatchItem::Custom(_) => {
                            let w = d.selected_custom_widget().unwrap();
                            if let Some(w) = w.as_any().downcast_ref::<DateEditor>() {
                                DuePatchItem::Custom(w.value())
                            } else {
                                panic!("Unexpected custom widget type")
                            }
                        }
                        _ => *p,
                    });
                }
                if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<ListDialog<Priority>>() {
                    new_priority = d.selected().cloned();
                }

                if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<CreateUpdateTaskDialog>() {
                    patch.provider_name = d.provider_name().await;
                    patch.project_id = d.project_id().await;
                    patch.task_patch = d.task_patch().await;
                    add_another_one_task = d.add_another_one();
                }
                self.dialog = None;

                if let Some(dh) = &self.draw_helper {
                    dh.write().await.hide_cursor();
                }
            }
        };

        if let Some(due) = new_due {
            self.change_due_date(&due).await;
        }

        if let Some(p) = new_priority {
            self.change_priority(&p).await;
        }

        if patch.is_valid() {
            self.create_or_update_task(&patch).await;
        }

        if need_to_update_view {
            self.update_task_info_view().await;
        }

        if add_another_one_task {
            self.show_add_task_dialog(None).await;
        }

        handled
    }
}

#[async_trait]
impl WidgetTrait for TasksWidget {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if self.list_state.selected().is_some_and(|idx| idx >= self.tasks.len()) {
            self.list_state.select(Some(0));
        }

        let changed = &self.changed_tasks;
        let mut title = format!("Tasks ({})", self.tasks.len());

        if !changed.is_empty() {
            title.push_str(format!(" (uncommitted count {}, use 'c'+'c' to commit them)", changed.len()).as_str());
        }

        let h = Header::new(title.as_str(), self.is_active(), Some(&self.activate_shortcut));
        h.block().render(area, buf);

        let mut y = area.y + 1;

        let mut selected = self.list_state.selected();
        if selected.is_some_and(|idx| idx >= self.tasks.len()) {
            selected = Some(0);
        }

        let skip_count = selected
            .map(|mut idx| {
                let mut height = y;
                while idx != 0 && height < area.height {
                    height += self.tasks[idx].size().height;
                    idx -= 1;
                }

                idx
            })
            .unwrap_or_default();

        for (i, w) in self.tasks.iter_mut().enumerate() {
            if i < skip_count || y > area.height {
                w.set_visible(false);
                continue;
            }

            w.set_visible(true);

            let is_row_selected = selected.is_some_and(|idx| idx == i);
            w.set_selected(is_row_selected);

            Text::from(if is_row_selected { ">" } else { " " })
                .style(default_style())
                .render(
                    Rect {
                        x: area.x,
                        y,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            w.set_pos(Position::new(area.x + 1, y));
            w.render(area, buf).await;

            let size = w.size();
            y += size.height;
        }

        Block::new()
            .style(style::default_style())
            .render(Rect::new(area.x, y, area.width, area.height + 2 - y), buf);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(self.tasks.len()).position(selected.unwrap_or_default());
        scrollbar.render(
            Rect {
                x: area.x,
                y: area.y + 1, // header
                width: area.width,
                height: area.height - 1,
            },
            buf,
            &mut scrollbar_state,
        );

        if self.dialog.is_some() {
            self.render_dialog(area, buf).await;
        }
    }

    fn set_draw_helper(&mut self, dh: DrawHelper) {
        self.draw_helper = Some(dh)
    }

    fn size(&self) -> Size {
        Size::default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for TasksWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TasksWidget")
    }
}

fn project_name(t: &dyn TaskTrait) -> String {
    t.project().map(|p| p.name()).unwrap_or_default()
}

fn replace_if<T>(op: &mut Option<T>, other: &Option<T>)
where
    T: Clone,
{
    if let Some(t) = other {
        op.replace(t.clone());
    }
}
