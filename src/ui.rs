use crate::filter;
use crate::task::{Task as TaskTrait, due_group};
use crate::{project, provider, task};
use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, ListItem, ListState, Paragraph, Widget};
use shortcut::{AcceptResult, Shortcut};
use std::collections::HashMap;
use std::fmt::Write;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OnceCell, RwLock};
mod filter_widget;
mod header;
mod hyperlink;
mod list;
mod selectable_list;
mod shortcut;
pub mod style;
mod task_description_widget;
mod tasks_widget;
use selectable_list::SelectableList;
use tokio_stream::StreamExt;

#[derive(Eq, PartialEq, Clone, Hash)]
enum AppBlock {
    Providers,
    Projects,
    Filter,
    TaskList,
    TaskDescription,
}

const BLOCK_ORDER: [AppBlock; 5] = [
    AppBlock::Providers,
    AppBlock::Projects,
    AppBlock::Filter,
    AppBlock::TaskList,
    AppBlock::TaskDescription,
];

trait AppBlockWidget {
    fn activate_shortcuts(&mut self) -> Vec<&mut Shortcut>;
    fn set_active(&mut self, is_active: bool);
}

#[derive(Default)]
struct KeyBuffer {
    keys: Vec<char>,
}

impl KeyBuffer {
    fn push(&mut self, key: char) -> Vec<char> {
        const MAX_KEYS_COUNT: usize = 2;
        if self.keys.len() == MAX_KEYS_COUNT {
            self.clear();
        }
        self.keys.push(key);
        self.keys.to_vec()
    }

    fn clear(&mut self) {
        self.keys.clear();
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl std::fmt::Display for KeyBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in &self.keys {
            f.write_char(*c)?
        }

        Ok(())
    }
}

pub struct App {
    should_exit: bool,
    reload_tasks: bool,
    providers: Arc<RwLock<SelectableList<Box<dyn provider::Provider>>>>,
    projects: Arc<RwLock<SelectableList<Box<dyn project::Project>>>>,
    current_block: AppBlock,

    filter_widget: Arc<RwLock<filter_widget::FilterWidget>>,
    tasks_widget: Arc<RwLock<tasks_widget::TasksWidget>>,
    task_description_widget: Arc<RwLock<task_description_widget::TaskDescriptionWidget>>,

    alert: Option<String>,
    app_blocks: HashMap<AppBlock, Arc<RwLock<dyn AppBlockWidget>>>,
    key_buffer: KeyBuffer,
}

#[allow(clippy::arc_with_non_send_sync)] // TODO: think how to remove this
impl App {
    pub fn new(providers: Vec<Box<dyn provider::Provider>>) -> Self {
        let mut s = Self {
            should_exit: false,
            reload_tasks: true,
            current_block: AppBlock::TaskList,
            providers: Arc::new(RwLock::new(
                SelectableList::new(providers, Some(0))
                    .add_all_item()
                    .shortcut(Shortcut::new(&['g', 'v'])),
            )),
            projects: Arc::new(RwLock::new(
                SelectableList::default()
                    .add_all_item()
                    .shortcut(Shortcut::new(&['g', 'p'])),
            )),
            filter_widget: filter_widget::FilterWidget::new(filter::Filter {
                states: vec![filter::FilterState::Uncompleted],
                due: vec![filter::Due::Today, filter::Due::Overdue],
            }),
            tasks_widget: Arc::new(RwLock::new(tasks_widget::TasksWidget::default())),
            task_description_widget: Arc::new(RwLock::new(
                task_description_widget::TaskDescriptionWidget::default(),
            )),
            alert: None,
            app_blocks: HashMap::new(),
            key_buffer: KeyBuffer::default(),
        };

        s.app_blocks
            .insert(AppBlock::Providers, s.providers.clone());
        s.app_blocks.insert(AppBlock::Projects, s.projects.clone());
        s.app_blocks
            .insert(AppBlock::TaskList, s.tasks_widget.clone());
        s.app_blocks
            .insert(AppBlock::TaskDescription, s.task_description_widget.clone());
        s.app_blocks
            .insert(AppBlock::Filter, s.filter_widget.clone());

        s
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        terminal.hide_cursor()?;

        self.tasks_widget.write().await.set_active(true);

        let period = Duration::from_secs_f32(1.0 / 60.0);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while !self.should_exit {
            if self.reload_tasks {
                self.load_tasks().await;
                self.reload_tasks = false;
            }

            tokio::select! {
                _ = interval.tick() => { self.draw(&mut terminal).await; },
                Some(Ok(event)) = events.next() => {
                    if let Event::Key(key) = event {
                        self.handle_key(key).await
                    }
                },
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
    }

    async fn load_projects(&mut self) {
        let mut projects = self.tasks_widget.read().await.tasks_projects();

        projects.sort_by(|l, r| {
            l.provider()
                .cmp(&r.provider())
                .then_with(|| l.name().cmp(&r.name()))
        });

        self.projects.write().await.set_items(projects);
        self.projects
            .write()
            .await
            .set_state(ListState::default().with_selected(Some(0)));
    }

    fn add_error(&mut self, message: &str) {
        self.alert = match self.alert.as_ref() {
            Some(s) => Some(format!("{}\n{}", s, message)),
            None => Some(message.to_string()),
        }
    }

    async fn selected_project_id(&self) -> Option<String> {
        self.projects.read().await.selected().map(|p| p.id())
    }

    async fn load_tasks(&mut self) {
        let mut all_tasks: Vec<Box<dyn task::Task>> = Vec::new();

        let selected_provider_name = if let Some(p) = self.providers.read().await.selected() {
            p.name()
        } else {
            String::new()
        };
        let mut errors = Vec::new();

        let project_id = self.selected_project_id().await;

        for p in self.providers.write().await.iter_mut() {
            if !selected_provider_name.is_empty() && p.name() != selected_provider_name {
                continue;
            }

            let tasks = p
                .tasks(None, &self.filter_widget.read().await.filter())
                .await;

            match tasks {
                Ok(t) => all_tasks.append(
                    &mut t
                        .iter()
                        .filter(|t| {
                            project_id.is_none()
                                || t.project().is_none()
                                || t.project().unwrap().id() == *project_id.as_ref().unwrap()
                        })
                        .map(|t| t.clone_boxed())
                        .collect::<Vec<Box<dyn TaskTrait>>>(),
                ),
                Err(err) => errors.push((p.name(), err)),
            }
        }

        for (provider_name, err) in errors {
            self.add_error(
                format!("Load provider {} projects failure: {}", provider_name, err).as_str(),
            )
        }

        all_tasks.sort_by(|l, r| {
            due_group(l.as_ref())
                .cmp(&due_group(r.as_ref()))
                .then_with(|| r.priority().cmp(&l.priority()))
                .then_with(|| l.due().cmp(&r.due()))
        });
        self.tasks_widget.write().await.set_tasks(all_tasks);

        if project_id.is_none() {
            self.load_projects().await;
        }
    }

    async fn handle_block_shortcuts(&mut self, key: &KeyEvent) -> bool {
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

        found_shortcut
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.handle_block_shortcuts(&key).await {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.alert.is_some() {
                    self.alert = None;
                } else {
                    self.should_exit = true;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                const BLOCKS: [AppBlock; 2] = [AppBlock::TaskList, AppBlock::TaskDescription];

                if BLOCKS.contains(&self.current_block) {
                    self.current_block = AppBlock::Providers;
                }

                self.update_activity_state().await;
            }
            KeyCode::Char('j') | KeyCode::Down => self.select_next().await,
            KeyCode::Char('k') | KeyCode::Up => self.select_previous().await,
            KeyCode::Char('g') | KeyCode::Home => self.select_first().await,
            KeyCode::Char('G') | KeyCode::End => self.select_last().await,
            KeyCode::Char('l') | KeyCode::Right => {
                const BLOCKS: [AppBlock; 3] =
                    [AppBlock::Providers, AppBlock::Projects, AppBlock::Filter];

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

        self.reload_tasks = true;
    }

    async fn change_check_state(&mut self) {
        match self.current_block {
            AppBlock::TaskList => {
                let result = self
                    .tasks_widget
                    .write()
                    .await
                    .change_check_state(&mut self.providers.write().await.iter_mut())
                    .await;
                if let Err(e) = result {
                    self.alert = Some(format!("Change state error: {e}"))
                }
                self.reload_tasks = true;
            }
            AppBlock::Filter => {
                self.filter_widget.write().await.change_check_state();
                self.projects.write().await.select_first();
                self.reload().await;
            }
            _ => {}
        }
    }

    async fn select_next_block(&mut self) {
        let cur_block_idx = BLOCK_ORDER
            .iter()
            .position(|x| *x == self.current_block)
            .unwrap();
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
        let cur_block_idx = BLOCK_ORDER
            .iter()
            .position(|x| *x == self.current_block)
            .unwrap();

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

    fn set_reload(&mut self) {
        match self.current_block {
            AppBlock::Providers | AppBlock::Projects | AppBlock::Filter => {
                self.reload_tasks = true;
            }
            AppBlock::TaskList => {}
            AppBlock::TaskDescription => {}
        }
    }

    async fn set_current_task(&mut self) {
        self.task_description_widget
            .write()
            .await
            .set_task(self.tasks_widget.read().await.selected_task());
    }

    async fn select_next(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.providers.write().await.select_next();
                self.projects.write().await.select_first();
            }
            AppBlock::Projects => self.projects.write().await.select_next(),
            AppBlock::Filter => self.filter_widget.write().await.select_next(),
            AppBlock::TaskList => {
                self.tasks_widget.write().await.select_next();
                self.set_current_task().await;
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    async fn select_previous(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.providers.write().await.select_previous();
                self.projects.write().await.select_first();
            }
            AppBlock::Projects => self.projects.write().await.select_previous(),
            AppBlock::Filter => self.filter_widget.write().await.select_previous(),
            AppBlock::TaskList => {
                self.tasks_widget.write().await.select_previous();
                self.set_current_task().await;
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    async fn select_first(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.write().await.select_first(),
            AppBlock::Projects => self.projects.write().await.select_first(),
            AppBlock::Filter => self.filter_widget.write().await.select_first(),
            AppBlock::TaskList => {
                self.tasks_widget.write().await.select_first();
                self.set_current_task().await;
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    async fn select_last(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.write().await.select_last(),
            AppBlock::Projects => self.projects.write().await.select_last(),
            AppBlock::Filter => self.filter_widget.write().await.select_last(),
            AppBlock::TaskList => {
                self.tasks_widget.write().await.select_last();
                self.set_current_task().await;
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }
}

/// Rendering logic for the app
impl App {
    async fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Length(50), Constraint::Fill(3)]).areas(main_area);

        let [providers_area, projects_area, filter_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ])
        .areas(left_area);
        let [list_area, task_description_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(20)]).areas(right_area);

        App::render_header(header_area, buf);
        self.render_footer(footer_area, buf);
        self.render_providers(providers_area, buf).await;
        self.render_projects(projects_area, buf).await;
        self.render_filters(filter_area, buf).await;
        self.render_tasks(list_area, buf).await;
        self.render_task_description(task_description_area, buf)
            .await;

        if let Some(alert) = &mut self.alert {
            let block = Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title("Alert!");
            let area = popup_area(area, 60, 20);
            Clear {}.render(area, buf);
            Paragraph::new(alert.to_string())
                .block(block)
                .centered()
                .render(area, buf);
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
                "Use ↓↑ to move up/down, Tab/BackTab to move between blocks. ",
                style::FOOTER_KEYS_HELP_COLOR,
            ),
            Span::styled("Current date/time: ", style::FOOTER_DATETIME_LABLE_FG),
            Span::styled(
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                style::FOOTER_DATETIME_FG,
            ),
        ];

        if !self.key_buffer.is_empty() {
            lines.push(Span::styled(" Keys: ", style::FOOTER_KEYS_LABLE_FG));
            lines.push(Span::styled(
                self.key_buffer.to_string(),
                style::FOOTER_KEYS_FG,
            ));
        }

        Paragraph::new(Line::from(lines))
            .centered()
            .render(area, buf);
        let link = hyperlink::Hyperlink::new("[Homepage]", "https://github.com/panter-dsd/tatuin");
        link.render(area, buf);
    }

    async fn render_providers(&mut self, area: Rect, buf: &mut Buffer) {
        self.providers.write().await.render(
            "Providers",
            |p| -> ListItem {
                ListItem::from(Span::styled(
                    format!("{} ({})", p.name(), p.type_name()),
                    p.color(),
                ))
            },
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

        let provider_color =
            |name: &str| provider_colors.iter().find(|(n, _)| n == name).unwrap().1;

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
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
