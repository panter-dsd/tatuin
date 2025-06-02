// SPDX-License-Identifier: MIT

use super::state::{StateSettings, StatefulObject};
use crate::filter;
use crate::state::{State, state_from_str, state_to_str};
use crate::{project, provider};
use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect, Size};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, ListItem, ListState, Paragraph, Widget, Wrap};
use regex::Regex;
use shortcut::{AcceptResult, Shortcut};
use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};
mod dialog;
mod filter_widget;
mod header;
mod hyperlink;
mod key_bindings_help_dialog;
mod key_buffer;
mod list;
mod selectable_list;
mod shortcut;
mod states_dialog;
pub mod style;
mod task_info_widget;
mod tasks_widget;
mod text_input_dialog;
use selectable_list::SelectableList;
use strum::{Display, EnumString};
use tokio_stream::StreamExt;

#[derive(Eq, PartialEq, Clone, Hash, Display, EnumString)]
enum AppBlock {
    Providers,
    Projects,
    Filter,
    TaskList,
    TaskInfo,
}

const BLOCK_ORDER: [AppBlock; 5] = [
    AppBlock::Providers,
    AppBlock::Projects,
    AppBlock::Filter,
    AppBlock::TaskList,
    AppBlock::TaskInfo,
];

#[async_trait]
trait AppBlockWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut>;
    fn set_active(&mut self, is_active: bool);

    async fn select_next(&mut self);
    async fn select_previous(&mut self);
    async fn select_first(&mut self);
    async fn select_last(&mut self);
}

pub struct App {
    should_exit: bool,
    providers: Arc<RwLock<SelectableList<Box<dyn provider::Provider>>>>,
    projects: Arc<RwLock<SelectableList<Box<dyn project::Project>>>>,
    current_block: AppBlock,

    filter_widget: Arc<RwLock<filter_widget::FilterWidget>>,
    tasks_widget: Arc<RwLock<tasks_widget::TasksWidget>>,
    task_description_widget: Arc<RwLock<task_info_widget::TaskInfoWidget>>,

    alert: Option<String>,
    app_blocks: HashMap<AppBlock, Arc<RwLock<dyn AppBlockWidget>>>,
    stateful_widgets: HashMap<AppBlock, Arc<RwLock<dyn StatefulObject>>>,
    key_buffer: key_buffer::KeyBuffer,

    select_first_shortcut: Shortcut,
    select_last_shortcut: Shortcut,
    load_state_shortcut: Shortcut,
    save_state_shortcut: Shortcut,
    commit_changes_shortcut: Shortcut,
    show_keybindings_help_shortcut: Shortcut,

    all_shortcuts: Vec<Arc<std::sync::RwLock<shortcut::SharedData>>>,

    dialog: Option<Box<dyn dialog::DialogTrait>>,

    settings: Arc<RwLock<Box<dyn StateSettings>>>,
}

#[allow(clippy::arc_with_non_send_sync)] // TODO: think how to remove this
impl App {
    pub fn new(providers: Vec<Box<dyn provider::Provider>>, settings: Box<dyn StateSettings>) -> Self {
        let mut s = Self {
            should_exit: false,
            current_block: AppBlock::TaskList,
            providers: Arc::new(RwLock::new(
                SelectableList::new(providers, Some(0))
                    .add_all_item()
                    .shortcut(Shortcut::new("Activate Providers block", &['g', 'v'])),
            )),
            projects: Arc::new(RwLock::new(
                SelectableList::default()
                    .add_all_item()
                    .shortcut(Shortcut::new("Activate Projects block", &['g', 'p'])),
            )),
            filter_widget: filter_widget::FilterWidget::new(filter::Filter {
                states: vec![filter::FilterState::Uncompleted],
                due: vec![filter::Due::Today, filter::Due::Overdue],
            }),
            tasks_widget: Arc::new(RwLock::new(tasks_widget::TasksWidget::default())),
            task_description_widget: Arc::new(RwLock::new(task_info_widget::TaskInfoWidget::default())),
            alert: None,
            app_blocks: HashMap::new(),
            stateful_widgets: HashMap::new(),
            key_buffer: key_buffer::KeyBuffer::default(),
            select_first_shortcut: Shortcut::new("Select first", &['g', 'g']),
            select_last_shortcut: Shortcut::new("Select last", &['G']),
            load_state_shortcut: Shortcut::new("Load state", &['s', 'l']),
            save_state_shortcut: Shortcut::new("Save the current state", &['s', 's']),
            commit_changes_shortcut: Shortcut::new("Commit changes", &['c', 'c']),
            show_keybindings_help_shortcut: Shortcut::new("Show help", &['?']),
            all_shortcuts: Vec::new(),
            dialog: None,
            settings: Arc::new(RwLock::new(settings)),
        };

        s.app_blocks.insert(AppBlock::Providers, s.providers.clone());
        s.app_blocks.insert(AppBlock::Projects, s.projects.clone());
        s.app_blocks.insert(AppBlock::TaskList, s.tasks_widget.clone());
        s.app_blocks
            .insert(AppBlock::TaskInfo, s.task_description_widget.clone());
        s.app_blocks.insert(AppBlock::Filter, s.filter_widget.clone());

        s.all_shortcuts.push(s.select_first_shortcut.internal_data());
        s.all_shortcuts.push(s.select_last_shortcut.internal_data());
        s.all_shortcuts.push(s.load_state_shortcut.internal_data());
        s.all_shortcuts.push(s.save_state_shortcut.internal_data());
        s.all_shortcuts.push(s.commit_changes_shortcut.internal_data());
        s.all_shortcuts.push(s.show_keybindings_help_shortcut.internal_data());

        s.stateful_widgets.insert(AppBlock::Providers, s.providers.clone());
        s.stateful_widgets.insert(AppBlock::Projects, s.projects.clone());
        s.stateful_widgets.insert(AppBlock::TaskList, s.tasks_widget.clone());
        s.stateful_widgets.insert(AppBlock::Filter, s.filter_widget.clone());

        s
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        for b in self.app_blocks.values_mut() {
            self.all_shortcuts
                .extend(b.write().await.activate_shortcuts().iter().map(|s| s.internal_data()));
        }

        if self.settings.read().await.states().is_empty() {
            // If there is no states, save the original as default
            self.save_state(None).await;
        }

        self.load_tasks().await;
        self.restore_state(None).await;

        terminal.hide_cursor()?;

        self.tasks_widget.write().await.set_active(true);

        let mut events = EventStream::new();

        let mut select_first_accepted = self.select_first_shortcut.subscribe_to_accepted();
        let mut select_last_accepted = self.select_last_shortcut.subscribe_to_accepted();
        let mut load_state_accepted = self.load_state_shortcut.subscribe_to_accepted();
        let mut save_state_accepted = self.save_state_shortcut.subscribe_to_accepted();
        let mut commit_changes_accepted = self.commit_changes_shortcut.subscribe_to_accepted();
        let mut show_keybindings_help_shortcut_accepted = self.show_keybindings_help_shortcut.subscribe_to_accepted();

        while !self.should_exit {
            if let Some(d) = &self.dialog {
                if d.should_be_closed() {
                    self.close_dialog().await;
                }
            }

            self.draw(&mut terminal).await;

            tokio::select! {
                Some(Ok(event)) = events.next() => {
                    if let Event::Key(key) = event {
                        self.handle_key(key).await;
                    }
                },
                _ = select_first_accepted.recv() => self.select_first().await,
                _ = select_last_accepted.recv() => self.select_last().await,
                _ = load_state_accepted.recv() => self.load_state().await,
                _ = save_state_accepted.recv() => self.save_state_as(),
                _ = commit_changes_accepted.recv() => self.commit_changes().await,
                _ = show_keybindings_help_shortcut_accepted.recv() => self.show_keybindings_help().await,
            }
        }
        Ok(())
    }

    async fn draw(&mut self, terminal: &mut DefaultTerminal) {
        let _ = terminal.autoresize();
        let mut frame = terminal.get_frame();
        let area = frame.area();
        let buf = frame.buffer_mut();
        self.render(area, buf).await;
        let _ = terminal.flush();
        terminal.swap_buffers();
        let _ = terminal.backend_mut().flush();
    }

    async fn load_projects(&mut self) {
        let mut projects = self.tasks_widget.read().await.tasks_projects();

        projects.sort_by(|l, r| l.provider().cmp(&r.provider()).then_with(|| l.name().cmp(&r.name())));

        self.projects.write().await.set_items(projects);
        self.projects
            .write()
            .await
            .set_state(ListState::default().with_selected(Some(0)));
    }

    fn add_error(&mut self, message: &str) {
        self.alert = match self.alert.as_ref() {
            Some(s) => Some(format!("{s}\n{message}")),
            None => Some(message.to_string()),
        }
    }

    async fn selected_project_id(&self) -> Option<String> {
        self.projects.read().await.selected().map(|p| p.id())
    }

    async fn load_tasks(&mut self) {
        let project_id = self.selected_project_id().await;

        let errors = self
            .tasks_widget
            .write()
            .await
            .load_tasks(
                &mut self.providers.write().await.iter_mut(),
                &self.filter_widget.read().await.filter(),
            )
            .await;

        for e in errors {
            self.add_error(e.to_string().as_str());
        }

        if project_id.is_none() {
            self.load_projects().await;
        }

        self.set_current_task().await;
    }

    async fn handle_shortcuts(&mut self, key: &KeyEvent) -> bool {
        let code = key.code.as_char();
        if code.is_none() {
            return false;
        }

        let code = code.unwrap();
        let mut found_shortcut = false;

        let keys = self.key_buffer.push(code);
        for (t, b) in &self.app_blocks {
            for s in b.write().await.activate_shortcuts() {
                match s.accept(&keys) {
                    AcceptResult::Accepted => {
                        self.key_buffer.clear();
                        self.current_block = t.clone();
                        found_shortcut = true;
                    }
                    AcceptResult::PartiallyAccepted => found_shortcut = true,
                    AcceptResult::NotAccepted => {}
                }
            }
        }

        self.update_activity_state().await;

        let shortcuts = vec![
            &mut self.select_first_shortcut,
            &mut self.select_last_shortcut,
            &mut self.load_state_shortcut,
            &mut self.save_state_shortcut,
            &mut self.commit_changes_shortcut,
            &mut self.show_keybindings_help_shortcut,
        ];
        for s in shortcuts {
            match s.accept(&keys) {
                AcceptResult::Accepted => {
                    self.key_buffer.clear();
                    found_shortcut = true;
                }
                AcceptResult::PartiallyAccepted => found_shortcut = true,
                AcceptResult::NotAccepted => {}
            }
        }

        found_shortcut
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if let Some(d) = &mut self.dialog {
            d.handle_key(key).await;
            return;
        }

        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.handle_shortcuts(&key).await {
            return;
        }

        self.key_buffer.clear();

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.alert.is_some() {
                    self.alert = None;
                } else {
                    self.should_exit = true;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                const BLOCKS: [AppBlock; 2] = [AppBlock::TaskList, AppBlock::TaskInfo];

                if BLOCKS.contains(&self.current_block) {
                    self.current_block = AppBlock::Providers;
                }

                self.update_activity_state().await;
            }
            KeyCode::Char('j') | KeyCode::Down => self.select_next().await,
            KeyCode::Char('k') | KeyCode::Up => self.select_previous().await,
            KeyCode::Char('l') | KeyCode::Right => {
                const BLOCKS: [AppBlock; 3] = [AppBlock::Providers, AppBlock::Projects, AppBlock::Filter];

                if BLOCKS.contains(&self.current_block) {
                    self.current_block = AppBlock::TaskList;
                }

                self.update_activity_state().await;
            }
            KeyCode::Tab => self.select_next_block().await,
            KeyCode::BackTab => self.select_previous_block().await,
            KeyCode::Char(' ') => self.change_check_state().await,
            KeyCode::Char('r') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.reload().await;
                }
            }
            _ => {}
        }
    }

    async fn update_activity_state(&mut self) {
        for (t, b) in &self.app_blocks {
            b.write().await.set_active(self.current_block == *t)
        }
    }

    async fn reload(&mut self) {
        for p in self.providers.write().await.iter_mut() {
            p.reload().await;
        }

        self.load_tasks().await;
    }

    async fn change_check_state(&mut self) {
        match self.current_block {
            AppBlock::TaskList => {
                let result = self.tasks_widget.write().await.change_check_state().await;
                if let Err(e) = result {
                    self.alert = Some(format!("Change state error: {e}"))
                }
            }
            AppBlock::Filter => {
                self.filter_widget.write().await.change_check_state();
                self.projects.write().await.select_first().await;
                self.reload().await;
            }
            _ => {}
        }
    }

    async fn select_next_block(&mut self) {
        let cur_block_idx = BLOCK_ORDER.iter().position(|x| *x == self.current_block).unwrap();
        let next_block_idx = if cur_block_idx == BLOCK_ORDER.len() - 1 {
            0
        } else {
            cur_block_idx + 1
        };

        match self.current_block {
            AppBlock::Projects => {
                self.current_block = AppBlock::Filter;
                self.filter_widget.write().await.set_active(true, false);
            }
            AppBlock::Filter => {
                if !self.filter_widget.write().await.next_block() {
                    self.current_block = AppBlock::TaskList;
                    self.filter_widget.write().await.set_active(false, false);
                }
            }
            _ => self.current_block = BLOCK_ORDER[next_block_idx].clone(),
        }

        self.update_activity_state().await;
    }

    async fn select_previous_block(&mut self) {
        let cur_block_idx = BLOCK_ORDER.iter().position(|x| *x == self.current_block).unwrap();

        let next_block_idx = if cur_block_idx == 0 {
            BLOCK_ORDER.len() - 1
        } else {
            cur_block_idx - 1
        };

        match self.current_block {
            AppBlock::TaskList => {
                self.current_block = BLOCK_ORDER[next_block_idx].clone();
                self.filter_widget.write().await.set_active(true, true);
            }
            AppBlock::Filter => {
                if !self.filter_widget.write().await.previous_block() {
                    self.current_block = BLOCK_ORDER[next_block_idx].clone();
                    self.filter_widget.write().await.set_active(false, true);
                }
            }
            _ => self.current_block = BLOCK_ORDER[next_block_idx].clone(),
        }

        self.update_activity_state().await;
    }

    async fn update_task_filter(&mut self) {
        let mut selected_providers = Vec::new();
        if let Some(p) = self.providers.read().await.selected() {
            selected_providers.push(p.name());
        }
        self.tasks_widget
            .write()
            .await
            .set_providers_filter(&selected_providers);

        let mut selected_projects = Vec::new();
        if let Some(p) = self.projects.read().await.selected() {
            selected_projects.push(p.name());
        }
        self.tasks_widget.write().await.set_projects_filter(&selected_projects);

        if self.current_block != AppBlock::Projects {
            self.load_projects().await;
        }
        self.load_tasks().await;
    }

    async fn set_current_task(&mut self) {
        self.task_description_widget
            .write()
            .await
            .set_task(self.tasks_widget.read().await.selected_task());
    }

    async fn on_selection_changed(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.projects.write().await.select_first().await;
                self.update_task_filter().await;
            }
            AppBlock::Projects => {
                self.update_task_filter().await;
            }
            AppBlock::TaskList => {
                self.set_current_task().await;
            }
            _ => {}
        }
    }

    async fn select_next(&mut self) {
        self.app_blocks
            .get_mut(&self.current_block)
            .unwrap()
            .write()
            .await
            .select_next()
            .await;
        self.on_selection_changed().await;
    }

    async fn select_previous(&mut self) {
        self.app_blocks
            .get_mut(&self.current_block)
            .unwrap()
            .write()
            .await
            .select_previous()
            .await;
        self.on_selection_changed().await;
    }

    async fn select_first(&mut self) {
        self.app_blocks
            .get_mut(&self.current_block)
            .unwrap()
            .write()
            .await
            .select_first()
            .await;
        self.on_selection_changed().await;
    }

    async fn select_last(&mut self) {
        self.app_blocks
            .get_mut(&self.current_block)
            .unwrap()
            .write()
            .await
            .select_last()
            .await;
        self.on_selection_changed().await;
    }

    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Length(50), Constraint::Fill(3)]).areas(main_area);

        let [providers_area, projects_area, filter_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(3), Constraint::Fill(1)]).areas(left_area);
        let [list_area, task_description_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(20)]).areas(right_area);

        App::render_header(header_area, buf);
        self.render_footer(footer_area, buf);
        self.render_providers(providers_area, buf).await;
        self.render_projects(projects_area, buf).await;
        self.render_filters(filter_area, buf).await;
        self.render_tasks(list_area, buf).await;
        self.render_task_description(task_description_area, buf).await;

        if let Some(alert) = &mut self.alert {
            let block = Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title("Alert!");
            let area = popup_area(area, Size::new(area.width / 2, 40));
            Clear {}.render(area, buf);
            Paragraph::new(alert.to_string())
                .block(block)
                .wrap(Wrap { trim: true })
                .render(area, buf);
        }

        if let Some(d) = &mut self.dialog {
            let size = d.size();
            let area = popup_area(area, size);
            Clear {}.render(area, buf);
            d.render(area, buf).await;
        }
    }

    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tatuin (Task Aggregator TUI for N providers)")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![
            Span::styled(
                "Use ↓↑ to move up/down, Tab/BackTab to move between blocks, ? for help. ",
                style::FOOTER_KEYS_HELP_COLOR,
            ),
            Span::styled("Current date/time: ", style::FOOTER_DATETIME_LABEL_FG),
            Span::styled(
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                style::FOOTER_DATETIME_FG,
            ),
        ];

        if !self.key_buffer.is_empty() {
            lines.push(Span::styled(" Keys: ", style::FOOTER_KEYS_LABEL_FG));
            lines.push(Span::styled(self.key_buffer.to_string(), style::FOOTER_KEYS_FG));
        }

        Paragraph::new(Line::from(lines)).centered().render(area, buf);
        let link = hyperlink::Hyperlink::new("[Homepage]", "https://github.com/panter-dsd/tatuin");
        link.render(area, buf);
    }

    async fn render_providers(&mut self, area: Rect, buf: &mut Buffer) {
        self.providers.write().await.render(
            "Providers",
            |p| -> ListItem { ListItem::from(Span::styled(format!("{} ({})", p.name(), p.type_name()), p.color())) },
            area,
            buf,
        );
    }

    async fn render_projects(&mut self, area: Rect, buf: &mut Buffer) {
        static PROVIDER_COLORS: OnceCell<Vec<(String, Color)>> = OnceCell::const_new();
        let provider_colors = PROVIDER_COLORS
            .get_or_init(async || {
                self.providers
                    .read()
                    .await
                    .iter()
                    .map(|p| (p.name(), p.color()))
                    .collect()
            })
            .await;

        let provider_color = |name: &str| provider_colors.iter().find(|(n, _)| n == name).unwrap().1;

        self.projects.write().await.render(
            "Projects",
            |p| -> ListItem {
                ListItem::from(Span::styled(
                    format!("{} ({})", p.name(), p.provider()),
                    provider_color(p.provider().as_str()),
                ))
            },
            area,
            buf,
        );
    }

    async fn render_tasks(&mut self, area: Rect, buf: &mut Buffer) {
        self.tasks_widget.write().await.render(area, buf)
    }

    async fn render_task_description(&mut self, area: Rect, buf: &mut Buffer) {
        self.task_description_widget.write().await.render(area, buf)
    }

    async fn render_filters(&mut self, area: Rect, buf: &mut Buffer) {
        self.filter_widget.write().await.render(area, buf)
    }

    fn save_state_as(&mut self) {
        let d = text_input_dialog::Dialog::new("State name", Regex::new(r"^[[:alpha:]]+[\[[:alpha:]\]\-_]*$").unwrap());
        self.dialog = Some(Box::new(d));
    }

    async fn save_state(&mut self, name: Option<&str>) {
        let mut state = State::new();
        let mut errors = Vec::new();

        for (block_name, w) in &self.stateful_widgets {
            let s = w.read().await.save();

            match state_to_str(&s) {
                Ok(v) => {
                    state.insert(block_name.to_string(), v);
                }
                Err(e) => {
                    errors.push(format!("serialize state of {block_name}: {e}"));
                }
            }
        }

        for e in errors {
            self.add_error(e.as_str());
        }

        let e = self.settings.write().await.save(name, state);
        if e.is_err() {
            self.add_error(format!("Save state error: {}", e.unwrap_err()).as_str());
        }
    }

    async fn restore_state(&mut self, name: Option<&str>) {
        for (block_name, st) in self.settings.read().await.load(name) {
            if let Ok(n) = AppBlock::from_str(block_name.as_str()) {
                if let Some(b) = self.stateful_widgets.get_mut(&n) {
                    if let Ok(st) = state_from_str(&st) {
                        b.write().await.restore(st);
                    }
                }
            }
        }

        self.update_task_filter().await;
    }

    async fn load_state(&mut self) {
        let d = states_dialog::Dialog::new(&self.settings).await;
        self.dialog = Some(Box::new(d));
    }

    async fn close_dialog(&mut self) {
        let d = self.dialog.take().unwrap();

        if let Some(d) = &d.as_any().downcast_ref::<states_dialog::Dialog>() {
            let mut state_to_restore = String::new();
            if let Some(s) = d.selected_state() {
                state_to_restore = s.clone();
            }
            if !state_to_restore.is_empty() {
                self.restore_state(Some(state_to_restore.as_str())).await;
            }
        }

        if let Some(d) = &d.as_any().downcast_ref::<text_input_dialog::Dialog>() {
            let t = d.text();
            if !t.is_empty() {
                self.save_state(Some(t.as_str())).await;
            }
        }
    }

    async fn commit_changes(&mut self) {
        if self.tasks_widget.read().await.has_changes() {
            let errors = self
                .tasks_widget
                .write()
                .await
                .commit_changes(&mut self.providers.write().await.iter_mut())
                .await;

            for e in errors {
                self.add_error(e.to_string().as_str());
            }

            self.load_tasks().await;
        }
    }

    async fn show_keybindings_help(&mut self) {
        let d = key_bindings_help_dialog::Dialog::new(&self.all_shortcuts);
        self.dialog = Some(Box::new(d));
    }
}

fn popup_area(area: Rect, size: Size) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(size.height)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(size.width)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
