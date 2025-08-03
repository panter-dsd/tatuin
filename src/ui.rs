// SPDX-License-Identifier: MIT

mod widgets;
use crate::async_jobs::AsyncJobStorage;

use super::{
    filter, project,
    provider::Provider,
    state::{State, StateSettings, StatefulObject, state_from_str, state_to_str},
    types::ArcRwLock,
    ui::{
        dialogs::{DialogTrait, KeyBindingsHelpDialog, StatesDialog, TextInputDialog},
        widgets::{WidgetStateTrait, WidgetTrait},
    },
};
use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    KeyboardEnhancementFlags, MouseEvent, PushKeyboardEnhancementFlags,
};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Position, Rect, Size},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, ListItem, ListState, Paragraph, Widget, Wrap},
};
use regex::Regex;
use shortcut::{AcceptResult, Shortcut};
use std::{
    collections::HashMap, hash::Hash, io::Write, slice::Iter, slice::IterMut, str::FromStr, sync::Arc, time::Duration,
};
use tasks_widget::ErrorLoggerTrait;
use tokio::sync::{OnceCell, RwLock, mpsc};
mod dialogs;
mod filter_widget;
mod header;
mod key_buffer;
mod list;
mod mouse_handler;
mod selectable_list;
mod shortcut;
pub mod style;
mod task_info_widget;
mod tasks_widget;
use crossterm::execute;
mod keyboard_handler;
use widgets::HyperlinkWidget;
mod draw_helper;
mod order_changer;
use mouse_handler::MouseHandler;
use selectable_list::SelectableList;
use strum::{Display, EnumString};
use tokio_stream::StreamExt;

#[derive(Debug, Eq, PartialEq, Clone, Hash, Display, EnumString)]
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
trait AppBlockWidget: WidgetTrait {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut>;
    fn shortcuts(&mut self) -> Vec<&mut Shortcut> {
        Vec::new()
    }

    async fn select_next(&mut self);
    async fn select_previous(&mut self);
    async fn select_first(&mut self);
    async fn select_last(&mut self);
}

struct DrawHelper {
    tx: mpsc::UnboundedSender<()>,
    set_pos_tx: mpsc::UnboundedSender<Option<Position>>,
    screen_size: Size,
}

impl DrawHelper {
    fn new(tx: mpsc::UnboundedSender<()>, set_pos_tx: mpsc::UnboundedSender<Option<Position>>) -> Self {
        Self {
            tx,
            set_pos_tx,
            screen_size: Size::default(),
        }
    }
}

impl draw_helper::DrawHelperTrait for DrawHelper {
    fn redraw(&mut self) {
        let _ = self.tx.send(());
    }
    fn set_cursor_pos(&mut self, pos: Position) {
        let _ = self.set_pos_tx.send(Some(pos));
    }
    fn hide_cursor(&mut self) {
        let _ = self.set_pos_tx.send(None);
    }

    fn screen_size(&self) -> Size {
        self.screen_size
    }

    fn set_screen_size(&mut self, s: Size) {
        self.screen_size = s
    }
}

struct ErrorLogger {
    errors: Vec<String>,
}

impl ErrorLogger {
    fn new() -> Self {
        Self { errors: Vec::new() }
    }

    fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    fn alert(&self) -> String {
        self.errors.join("\n")
    }

    fn clear(&mut self) {
        self.errors.clear();
    }
}

impl ErrorLoggerTrait for ErrorLogger {
    fn add_error(&mut self, message: &str) {
        self.errors.push(message.to_string())
    }
}

pub struct App {
    should_exit: bool,
    providers: ArcRwLock<SelectableList<Provider>>,
    projects: ArcRwLock<SelectableList<Box<dyn project::Project>>>,
    async_jobs: ArcRwLock<SelectableList<String>>,
    current_block: AppBlock,
    draw_helper: Option<draw_helper::DrawHelper>,
    async_jobs_storage: ArcRwLock<AsyncJobStorage>,

    filter_widget: ArcRwLock<filter_widget::FilterWidget>,
    tasks_widget: ArcRwLock<tasks_widget::TasksWidget>,
    task_info_widget: ArcRwLock<task_info_widget::TaskInfoWidget>,
    home_link: HyperlinkWidget,

    error_logger: ArcRwLock<ErrorLogger>,
    app_blocks: HashMap<AppBlock, ArcRwLock<dyn AppBlockWidget>>,
    stateful_widgets: HashMap<AppBlock, ArcRwLock<dyn StatefulObject>>,
    key_buffer: key_buffer::KeyBuffer,

    select_first_shortcut: Shortcut,
    select_last_shortcut: Shortcut,
    load_state_shortcut: Shortcut,
    save_state_shortcut: Shortcut,
    show_keybindings_help_shortcut: Shortcut,

    all_shortcuts: Vec<Arc<std::sync::RwLock<shortcut::SharedData>>>,

    dialog: Option<Box<dyn DialogTrait>>,

    settings: ArcRwLock<Box<dyn StateSettings>>,
    cursor_pos: Option<Position>,
}

impl tasks_widget::ProvidersStorage for SelectableList<Provider> {
    fn iter_mut<'a>(&'a mut self) -> IterMut<'a, Provider> {
        self.iter_mut()
    }
    fn iter<'a>(&'a self) -> Iter<'a, Provider> {
        self.iter()
    }
    fn provider(&self, name: &str) -> Provider {
        self.iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("Provider with name='{name}' not found"))
            .clone()
    }
}

#[async_trait]
impl tasks_widget::TaskInfoViewerTrait for task_info_widget::TaskInfoWidget {
    async fn set_task(&mut self, task: Option<Box<dyn crate::task::Task>>) {
        self.set_task(task).await;
    }
}

#[allow(clippy::arc_with_non_send_sync)] // TODO: think how to remove this
impl App {
    pub async fn new(providers: Vec<Provider>, settings: Box<dyn StateSettings>) -> Self {
        let providers_widget = Arc::new(RwLock::new(
            SelectableList::new(providers, Some(0))
                .add_all_item()
                .shortcut(Shortcut::new("Activate Providers block", &['g', 'v'])),
        ));
        let error_logger = Arc::new(RwLock::new(ErrorLogger::new()));
        let task_info_widget = Arc::new(RwLock::new(task_info_widget::TaskInfoWidget::default()));
        let async_jobs_storage = Arc::new(RwLock::new(AsyncJobStorage::default()));
        let mut s = Self {
            should_exit: false,
            current_block: AppBlock::TaskList,
            draw_helper: None,
            async_jobs_storage: async_jobs_storage.clone(),
            providers: providers_widget.clone(),
            projects: Arc::new(RwLock::new(
                SelectableList::default()
                    .add_all_item()
                    .shortcut(Shortcut::new("Activate Projects block", &['g', 'p'])),
            )),
            async_jobs: Arc::new(RwLock::new(SelectableList::new(Vec::new(), None))),
            filter_widget: filter_widget::FilterWidget::new(filter::Filter {
                states: vec![filter::FilterState::Uncompleted],
                due: vec![filter::Due::Today, filter::Due::Overdue],
            }),
            tasks_widget: tasks_widget::TasksWidget::new(
                providers_widget.clone(),
                error_logger.clone(),
                task_info_widget.clone(),
                async_jobs_storage.clone(),
            )
            .await,
            task_info_widget,
            home_link: HyperlinkWidget::new("[Homepage]", "https://github.com/panter-dsd/tatuin"),
            error_logger: error_logger.clone(),
            app_blocks: HashMap::new(),
            stateful_widgets: HashMap::new(),
            key_buffer: key_buffer::KeyBuffer::default(),
            select_first_shortcut: Shortcut::new("Select first", &['g', 'g']).global(),
            select_last_shortcut: Shortcut::new("Select last", &['G']).global(),
            load_state_shortcut: Shortcut::new("Load state", &['s', 'l']).global(),
            save_state_shortcut: Shortcut::new("Save the current state", &['s', 's']).global(),
            show_keybindings_help_shortcut: Shortcut::new("Show help", &['?']).global(),
            all_shortcuts: Vec::new(),
            dialog: None,
            settings: Arc::new(RwLock::new(settings)),
            cursor_pos: None,
        };

        s.app_blocks.insert(AppBlock::Providers, s.providers.clone());
        s.app_blocks.insert(AppBlock::Projects, s.projects.clone());
        s.app_blocks.insert(AppBlock::TaskList, s.tasks_widget.clone());
        s.app_blocks.insert(AppBlock::TaskInfo, s.task_info_widget.clone());
        s.app_blocks.insert(AppBlock::Filter, s.filter_widget.clone());

        s.all_shortcuts.push(s.select_first_shortcut.internal_data());
        s.all_shortcuts.push(s.select_last_shortcut.internal_data());
        s.all_shortcuts.push(s.load_state_shortcut.internal_data());
        s.all_shortcuts.push(s.save_state_shortcut.internal_data());
        s.all_shortcuts.push(s.show_keybindings_help_shortcut.internal_data());

        s.stateful_widgets.insert(AppBlock::Providers, s.providers.clone());
        s.stateful_widgets.insert(AppBlock::Projects, s.projects.clone());
        s.stateful_widgets.insert(AppBlock::Filter, s.filter_widget.clone());

        s
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        execute!(std::io::stdout(), EnableMouseCapture)?;
        execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;

        for b in self.app_blocks.values_mut() {
            let mut b = b.write().await;
            self.all_shortcuts.extend(b.activate_shortcuts().iter().map(|s| {
                let d = s.internal_data();
                d.write().unwrap().is_global = true;
                d
            }));
            self.all_shortcuts
                .extend(b.shortcuts().iter().map(|s| s.internal_data()));
        }

        if self.settings.read().await.states().is_empty() {
            // If there is no states, save the original as default
            self.save_state(None).await;
        }

        self.restore_state(None).await;

        self.tasks_widget.write().await.set_active(true);

        let (redraw_tx, mut redraw_rx) = mpsc::unbounded_channel::<()>();
        let (set_cursor_pos_tx, mut set_cursor_pos_rx) = mpsc::unbounded_channel::<Option<Position>>();
        let dh = {
            let mut d: Box<dyn draw_helper::DrawHelperTrait> = Box::new(DrawHelper::new(redraw_tx, set_cursor_pos_tx));
            d.set_screen_size(terminal.get_frame().area().as_size());
            Arc::new(RwLock::new(d))
        };

        self.draw_helper = Some(dh.clone());

        for b in self.app_blocks.values_mut() {
            b.write().await.set_draw_helper(dh.clone());
        }

        let redraw_period = Duration::from_secs(60); // every minute
        let mut redraw_interval = tokio::time::interval(redraw_period);
        let mut events = EventStream::new();

        let mut select_first_accepted = self.select_first_shortcut.subscribe_to_accepted();
        let mut select_last_accepted = self.select_last_shortcut.subscribe_to_accepted();
        let mut load_state_accepted = self.load_state_shortcut.subscribe_to_accepted();
        let mut save_state_accepted = self.save_state_shortcut.subscribe_to_accepted();
        let mut show_keybindings_help_shortcut_accepted = self.show_keybindings_help_shortcut.subscribe_to_accepted();
        let mut on_tasks_changed = self.tasks_widget.read().await.subscribe_on_changes();
        let mut on_jobs_changed = self.async_jobs_storage.read().await.subscribe_on_changes();

        let mut screen_size = dh.read().await.screen_size();
        while !self.should_exit {
            if let Some(d) = &self.dialog {
                if d.should_be_closed() {
                    self.close_dialog().await;
                }
            }

            {
                let ss = terminal.get_frame().area().as_size();
                if ss != screen_size {
                    dh.write().await.set_screen_size(ss);
                    screen_size = ss;
                }
            }

            self.draw(&mut terminal).await;

            tokio::select! {
                _ = redraw_rx.recv() => {},
                _ = redraw_interval.tick() => {},
                Some(pos) = set_cursor_pos_rx.recv() => {
                    self.cursor_pos = pos;
                },
                Some(Ok(event)) = events.next() => {
                    match event {
                        Event::Key(key) => {
                            self.handle_key(key).await;
                        },
                        Event::Mouse(ev) => {
                            self.handle_mouse(ev).await;
                        },
                        _ => {},
                    };
                },
                _ = on_tasks_changed.recv() => {
                    if self.selected_project_id().await.is_none() {
                        self.load_projects().await;
                    }
                },
                _ = on_jobs_changed.recv() => {
                    self.async_jobs.write().await.set_items(self.async_jobs_storage.read().await.jobs());
                },
                _ = select_first_accepted.recv() => self.select_first().await,
                _ = select_last_accepted.recv() => self.select_last().await,
                _ = load_state_accepted.recv() => self.load_state().await,
                _ = save_state_accepted.recv() => self.save_state_as(),
                _ = show_keybindings_help_shortcut_accepted.recv() => self.show_keybindings_help().await,
            }
        }

        execute!(std::io::stdout(), DisableMouseCapture)?;
        Ok(())
    }

    async fn handle_mouse(&mut self, ev: MouseEvent) {
        for b in self.app_blocks.values_mut() {
            b.write().await.handle_mouse(&ev).await;
        }
        self.home_link.handle_mouse(&ev).await;
    }

    async fn draw(&mut self, terminal: &mut DefaultTerminal) {
        let _ = terminal.autoresize();
        let mut frame = terminal.get_frame();
        let area = frame.area();
        let buf = frame.buffer_mut();
        self.render(area, buf).await;
        let _ = terminal.flush();

        match self.cursor_pos {
            Some(pos) => {
                let _ = terminal.show_cursor();
                let _ = terminal.set_cursor_position(pos);
            }
            None => {
                let _ = terminal.hide_cursor();
            }
        }
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

    async fn add_error(&mut self, message: &str) {
        self.error_logger.write().await.add_error(message);
    }

    async fn selected_project_id(&self) -> Option<String> {
        self.projects.read().await.selected().map(|p| p.id())
    }

    async fn load_tasks(&mut self) {
        self.tasks_widget
            .write()
            .await
            .load_tasks(&self.filter_widget.read().await.filter())
            .await;
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
            let mut b = b.write().await;
            for s in b.activate_shortcuts() {
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
            for s in b.shortcuts() {
                if s.is_global() || self.current_block == *t {
                    match s.accept(&keys) {
                        AcceptResult::Accepted => {
                            self.key_buffer.clear();
                            found_shortcut = true;
                        }
                        AcceptResult::PartiallyAccepted => found_shortcut = true,
                        AcceptResult::NotAccepted => {}
                    }
                }
            }
        }

        self.update_activity_state().await;

        let shortcuts = vec![
            &mut self.select_first_shortcut,
            &mut self.select_last_shortcut,
            &mut self.load_state_shortcut,
            &mut self.save_state_shortcut,
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

        let handled_by_current_block = self
            .app_blocks
            .get_mut(&self.current_block)
            .unwrap()
            .write()
            .await
            .handle_key(key)
            .await;
        if handled_by_current_block {
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
                if self.error_logger.read().await.is_empty() {
                    self.should_exit = true;
                } else {
                    self.error_logger.write().await.clear();
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
            p.provider.write().await.reload().await;
        }

        self.tasks_widget.write().await.reload().await;
        self.load_tasks().await;
    }

    async fn change_check_state(&mut self) {
        if self.current_block == AppBlock::Filter {
            self.filter_widget.write().await.change_check_state();
            self.projects.write().await.select_first().await;
            self.reload().await;
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
            selected_providers.push(p.name.clone());
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

    async fn on_selection_changed(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.projects.write().await.select_first().await;
                self.update_task_filter().await;
            }
            AppBlock::Projects => {
                self.update_task_filter().await;
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

        let have_async_jobs = !self.async_jobs_storage.read().await.is_empty();

        let [providers_area, projects_area, async_jobs_area, filter_area] = Layout::vertical([
            Constraint::Length(self.providers.read().await.len() as u16 + 1 + 1),
            Constraint::Fill(3),
            Constraint::Fill(if have_async_jobs { 1 } else { 0 }),
            Constraint::Length(self.filter_widget.read().await.size().height),
        ])
        .areas(left_area);

        let [list_area, task_description_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(20)]).areas(right_area);

        App::render_header(header_area, buf);
        self.render_providers(providers_area, buf).await;
        self.render_projects(projects_area, buf).await;
        self.async_jobs.write().await.render(
            "Async jobs",
            |name| -> ListItem { ListItem::from(name.clone()) },
            async_jobs_area,
            buf,
        );
        self.render_filters(filter_area, buf).await;
        self.render_task_description(task_description_area, buf).await;
        self.render_tasks(list_area, buf).await;
        self.render_footer(footer_area, buf).await;

        if !self.error_logger.read().await.is_empty() {
            let block = Block::bordered()
                .border_style(style::DEFAULT_STYLE.fg(Color::Red))
                .title("Alert!");
            let area = popup_area(area, Size::new(area.width / 2, 40));
            Clear {}.render(area, buf);
            Paragraph::new(self.error_logger.read().await.alert())
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

    async fn render_footer(&mut self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![
            Span::styled(
                "Use ↓↑ to move up/down, Tab/BackTab to move between blocks, ? for help. ",
                style::FOOTER_KEYS_HELP_COLOR,
            ),
            Span::styled(
                "Current date/time: ",
                style::DEFAULT_STYLE.fg(style::FOOTER_DATETIME_LABEL_FG),
            ),
            Span::styled(
                chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                style::FOOTER_DATETIME_FG,
            ),
        ];

        if !self.key_buffer.is_empty() {
            lines.push(Span::styled(" Keys: ", style::FOOTER_KEYS_LABEL_FG));
            lines.push(Span::styled(self.key_buffer.to_string(), style::FOOTER_KEYS_FG));
        }

        Paragraph::new(Line::from(lines)).centered().render(area, buf);
        self.render_home_link(area, buf).await;
    }

    async fn render_home_link(&mut self, area: Rect, buf: &mut Buffer) {
        let s = self.home_link.size();
        let pos = Position::new(area.x + area.width - s.width, area.y + area.height - s.height);
        self.home_link.set_pos(pos);
        self.home_link.render(area, buf).await;
    }

    async fn render_providers(&mut self, area: Rect, buf: &mut Buffer) {
        self.providers.write().await.render(
            "Providers",
            |p| -> ListItem { ListItem::from(Span::styled(format!("{} ({})", p.name, p.type_name), p.color)) },
            area,
            buf,
        );
    }

    async fn render_projects(&mut self, area: Rect, buf: &mut Buffer) {
        static PROVIDER_COLORS: OnceCell<Vec<(String, Color)>> = OnceCell::const_new();
        let provider_colors = PROVIDER_COLORS
            .get_or_init(async || {
                let mut result = Vec::new();
                for p in self.providers.read().await.iter() {
                    result.push((p.name.clone(), p.color));
                }
                result
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
        self.tasks_widget.write().await.render(area, buf).await
    }

    async fn render_task_description(&mut self, area: Rect, buf: &mut Buffer) {
        self.task_info_widget.write().await.render(area, buf).await
    }

    async fn render_filters(&mut self, area: Rect, buf: &mut Buffer) {
        self.filter_widget.write().await.render(area, buf).await
    }

    fn save_state_as(&mut self) {
        let mut d = TextInputDialog::new("State name", Regex::new(r"^[[:alpha:]]+[\[[:alpha:]\]\-_]*$").unwrap());
        d.set_draw_helper(self.draw_helper.as_ref().unwrap().clone());
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
            self.add_error(e.as_str()).await;
        }

        let e = self.settings.write().await.save(name, state);
        if e.is_err() {
            self.add_error(format!("Save state error: {}", e.unwrap_err()).as_str())
                .await;
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
        let d = StatesDialog::new(&self.settings).await;
        self.dialog = Some(Box::new(d));
    }

    async fn close_dialog(&mut self) {
        let d = self.dialog.take().unwrap();

        if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<StatesDialog>() {
            let mut state_to_restore = String::new();
            if let Some(s) = d.selected_state() {
                state_to_restore = s.clone();
            }
            if !state_to_restore.is_empty() {
                self.restore_state(Some(state_to_restore.as_str())).await;
            }
        }

        if let Some(d) = DialogTrait::as_any(d.as_ref()).downcast_ref::<TextInputDialog>() {
            let t = d.text();
            if !t.is_empty() {
                self.save_state(Some(t.as_str())).await;
            }
        }
    }

    async fn show_keybindings_help(&mut self) {
        let current_block = self.app_blocks.get_mut(&self.current_block).unwrap();
        let d = KeyBindingsHelpDialog::new(
            &current_block
                .write()
                .await
                .shortcuts()
                .iter()
                .filter(|s| !s.is_global())
                .map(|s| s.internal_data())
                .collect::<Vec<Arc<std::sync::RwLock<shortcut::SharedData>>>>(),
            &self
                .all_shortcuts
                .iter()
                .filter(|s| s.read().unwrap().is_global)
                .cloned()
                .collect::<Vec<Arc<std::sync::RwLock<shortcut::SharedData>>>>(),
        );
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
