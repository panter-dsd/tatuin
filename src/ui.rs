use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::{project, provider, task};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget,
    Widget,
};
use ratatui::{DefaultTerminal, symbols};
mod filter_widget;
mod style;
mod task_description_widget;
use std::cmp::Ordering;

#[derive(Eq, PartialEq, Clone)]
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

struct SelectableList<T> {
    items: Vec<T>,
    state: ListState,
}

impl<T> SelectableList<T> {
    fn new(v: Vec<T>, selected: Option<usize>) -> Self {
        Self {
            items: v,
            state: ListState::default().with_selected(selected),
        }
    }
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: ListState::default(),
        }
    }
}

pub struct App {
    should_exit: bool,
    reload_tasks: bool,
    providers: SelectableList<Box<dyn provider::Provider>>,
    projects: SelectableList<Box<dyn project::Project>>,
    tasks: SelectableList<Box<dyn task::Task>>,
    current_block: AppBlock,

    filter_widget: filter_widget::FilterWidget,
    task_description_widget: task_description_widget::TaskDescriptionWidget,

    alert: Option<String>,
}

impl App {
    pub fn new(providers: Vec<Box<dyn provider::Provider>>) -> Self {
        Self {
            should_exit: false,
            reload_tasks: true,
            current_block: AppBlock::TaskList,
            providers: SelectableList::new(providers, Some(0)),
            projects: SelectableList::default(),
            tasks: SelectableList::default(),
            filter_widget: filter_widget::FilterWidget::new(filter::Filter {
                states: vec![filter::FilterState::Uncompleted],
                due: vec![filter::Due::Today, filter::Due::Overdue],
            }),
            task_description_widget: task_description_widget::TaskDescriptionWidget::default(),
            alert: None,
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_exit {
            if self.reload_tasks {
                self.load_tasks().await;
                self.reload_tasks = false;
            }

            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key).await;
            }
        }
        Ok(())
    }

    async fn load_projects(&mut self) {
        let mut projects: Vec<Box<dyn ProjectTrait>> = Vec::new();

        for t in &self.tasks.items {
            if let Some(tp) = t.project() {
                let it = projects
                    .iter()
                    .find(|p| p.id() == tp.id() && p.provider() == tp.provider());
                if it.is_none() {
                    projects.push(t.project().unwrap().clone_boxed());
                }
            }
        }

        projects.sort_by(|l, r| {
            l.provider()
                .cmp(&r.provider())
                .then_with(|| l.name().cmp(&r.name()))
        });

        self.projects.items = projects;
        self.projects.state = ListState::default().with_selected(Some(0));
    }

    fn add_error(&mut self, message: &str) {
        self.alert = match self.alert.as_ref() {
            Some(s) => Some(format!("{}\n{}", s, message)),
            None => Some(message.to_string()),
        }
    }

    async fn load_tasks(&mut self) {
        let mut tasks: Vec<Box<dyn task::Task>> = Vec::new();
        let selected_provider_idx = std::cmp::min(
            self.providers.state.selected().unwrap_or_default(),
            self.providers.items.len(),
        );

        let mut errors = Vec::new();

        for (i, p) in self.providers.items.iter_mut().enumerate() {
            let project = if let Some(idx) = self.projects.state.selected() {
                if idx < 1 {
                    None
                } else {
                    Some(self.projects.items[idx - 1].as_ref().clone_boxed())
                }
            } else {
                None
            };

            if selected_provider_idx != 0 && i != selected_provider_idx - 1 {
                continue;
            }

            match p.tasks(project, &self.filter_widget.filter()).await {
                Ok(mut prj) => tasks.append(&mut prj),
                Err(err) => errors.push((p.name(), err)),
            }
        }

        for (provider_name, err) in errors {
            self.add_error(
                format!("Load provider {} projects failure: {}", provider_name, err).as_str(),
            )
        }

        tasks.sort_by_key(|k| k.due());
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

        self.set_current_task();
        self.load_projects().await;
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
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
            }
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.select_last(),
            KeyCode::Char('l') | KeyCode::Right => {
                const BLOCKS: [AppBlock; 3] =
                    [AppBlock::Providers, AppBlock::Projects, AppBlock::Filter];

                if BLOCKS.contains(&self.current_block) {
                    self.current_block = AppBlock::TaskList;
                }
            }
            KeyCode::Tab => self.select_next_block(),
            KeyCode::BackTab => self.select_previous_block(),
            KeyCode::Char(' ') => self.change_check_state().await,
            KeyCode::Char('r') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.reload().await;
                }
            }
            _ => {}
        }
    }

    async fn reload(&mut self) {
        for p in &mut self.providers.items {
            p.reload().await;
        }

        self.reload_tasks = true;
    }

    async fn change_check_state(&mut self) {
        match self.current_block {
            AppBlock::Providers | AppBlock::Projects => {} //TODO: implement
            AppBlock::TaskList => {
                if self.tasks.state.selected().is_none() {
                    return;
                }

                let t = &self.tasks.items[self.tasks.state.selected().unwrap()];
                let provider_idx = self
                    .providers
                    .items
                    .iter()
                    .position(|p| p.name() == t.provider())
                    .unwrap();
                let provider = &mut self.providers.items[provider_idx];
                let st = match t.state() {
                    task::State::Completed => task::State::Uncompleted,
                    task::State::Uncompleted | task::State::InProgress => task::State::Completed,
                    task::State::Unknown(_) => task::State::Completed,
                };
                if let Err(e) = provider.change_task_state(t.as_ref(), st).await {
                    self.alert = Some(format!("Change state error: {e}"))
                }
                self.reload_tasks = true;
            }
            AppBlock::Filter => {
                self.filter_widget.change_check_state();
                self.reload_tasks = true;
            }
            AppBlock::TaskDescription => {}
        }
    }

    fn select_next_block(&mut self) {
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
                self.filter_widget.set_active(true, false);
            }
            AppBlock::Filter => {
                if !self.filter_widget.next_block() {
                    self.current_block = AppBlock::TaskList;
                    self.filter_widget.set_active(false, false);
                }
            }
            _ => self.current_block = BLOCK_ORDER[next_block_idx].clone(),
        }
    }

    fn select_previous_block(&mut self) {
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
                self.filter_widget.set_active(true, true);
            }
            AppBlock::Filter => {
                if !self.filter_widget.previous_block() {
                    self.current_block = BLOCK_ORDER[next_block_idx].clone();
                    self.filter_widget.set_active(false, true);
                }
            }
            _ => self.current_block = BLOCK_ORDER[next_block_idx].clone(),
        }
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

    fn set_current_task(&mut self) {
        if self.tasks.state.selected().is_some() && !self.tasks.items.is_empty() {
            let selected_idx = std::cmp::min(
                self.tasks.state.selected().unwrap_or_default(),
                self.tasks.items.len() - 1,
            );
            let t = &self.tasks.items[selected_idx];
            self.task_description_widget.set_task(Some(t.clone_boxed()));
        } else {
            self.task_description_widget.set_task(None);
        }
    }

    fn select_next(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select_next(),
            AppBlock::Projects => self.projects.state.select_next(),
            AppBlock::Filter => self.filter_widget.select_next(),
            AppBlock::TaskList => {
                self.tasks.state.select_next();
                self.set_current_task();
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_previous(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select_previous(),
            AppBlock::Projects => self.projects.state.select_previous(),
            AppBlock::Filter => self.filter_widget.select_previous(),
            AppBlock::TaskList => {
                self.tasks.state.select_previous();
                self.set_current_task();
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_first(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select_first(),
            AppBlock::Projects => self.projects.state.select_first(),
            AppBlock::Filter => self.filter_widget.select_first(),
            AppBlock::TaskList => self.tasks.state.select_first(),
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_last(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select_last(),
            AppBlock::Projects => self.projects.state.select_last(),
            AppBlock::Filter => self.filter_widget.select_last(),
            AppBlock::TaskList => self.tasks.state.select_last(),
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Length(50), Constraint::Fill(3)]).areas(main_area);

        let [
            providers_area,
            projects_area,
            filter_header_area,
            filter_area,
        ] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(left_area);
        let [list_area, task_description_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(20)]).areas(right_area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);
        self.render_tasks(list_area, buf);
        self.render_providers(providers_area, buf);
        self.render_projects(projects_area, buf);
        self.render_filter(filter_header_area, filter_area, buf);
        self.render_task_description(task_description_area, buf);

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
}

/// Rendering logic for the app
impl App {
    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tatuin (Task Aggregator TUI for N providers)")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, ← to unselect, → to change status, g/G to go top/bottom.")
            .centered()
            .render(area, buf);
    }

    fn block_style(&self, b: AppBlock) -> Style {
        if self.current_block == b {
            return style::ACTIVE_BLOCK_STYLE;
        }

        style::INACTIVE_BLOCK_STYLE
    }

    fn prepare_render_list<'a>(
        &self,
        title: &'a str,
        block: AppBlock,
        items: &'a [ListItem],
    ) -> List<'a> {
        let block = Block::new()
            .title(Line::raw(title).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(self.block_style(block))
            .bg(style::NORMAL_ROW_BG);

        List::new(items.to_vec())
            .block(block)
            .highlight_style(style::SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always)
    }

    fn render_providers(&mut self, area: Rect, buf: &mut Buffer) {
        let mut items: Vec<ListItem> = self
            .providers
            .items
            .iter()
            .map(|p| ListItem::from(format!("{} ({})", p.name(), p.type_name())))
            .collect();

        items.insert(0, ListItem::from("All"));
        StatefulWidget::render(
            self.prepare_render_list("Providers", AppBlock::Providers, &items),
            area,
            buf,
            &mut self.providers.state,
        );
    }

    fn render_projects(&mut self, area: Rect, buf: &mut Buffer) {
        let mut items: Vec<ListItem> = self
            .projects
            .items
            .iter()
            .map(|p| ListItem::from(format!("{} ({})", p.name(), p.provider())))
            .collect();

        items.insert(0, ListItem::from("All"));
        StatefulWidget::render(
            self.prepare_render_list("Projects", AppBlock::Projects, &items),
            area,
            buf,
            &mut self.projects.state,
        );
    }

    fn render_filter(&mut self, header_area: Rect, area: Rect, buf: &mut Buffer) {
        Block::new()
            .title(Line::raw("Filter").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(self.block_style(AppBlock::Filter))
            .bg(style::NORMAL_ROW_BG)
            .render(header_area, buf);
        self.filter_widget.render(area, buf);
    }

    fn render_tasks(&mut self, area: Rect, buf: &mut Buffer) {
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
                    Span::styled(t.place(), Style::default().fg(Color::Green)),
                    Span::from(")"),
                ]);

                ListItem::from(mixed_line)
            })
            .collect();

        StatefulWidget::render(
            self.prepare_render_list(
                format!("Tasks ({})", items.len()).as_str(),
                AppBlock::TaskList,
                &items,
            ),
            area,
            buf,
            &mut self.tasks.state,
        );
    }

    fn render_task_description(&mut self, area: Rect, buf: &mut Buffer) {
        self.task_description_widget.render(area, buf)
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
