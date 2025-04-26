use crate::filter;
use crate::{project, provider, task};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::palette::tailwind::{BLUE, GREEN, SLATE};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
    StatefulWidget, Widget, Wrap,
};
use ratatui::{DefaultTerminal, symbols};
mod filter_widget;

const ACTIVE_BLOCK_STYLE: Style = Style::new().fg(SLATE.c100).bg(GREEN.c800);
const INACTIVE_BLOCK_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;

#[derive(Eq, PartialEq)]
enum AppBlock {
    Providers,
    Projects,
    Filter,
    TaskList,
    TaskDescription,
}

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
    reload_projects: bool,
    reload_tasks: bool,
    providers: SelectableList<Box<dyn provider::Provider>>,
    projects: SelectableList<Box<dyn project::Project>>,
    tasks: SelectableList<Box<dyn task::Task>>,
    current_block: AppBlock,
    filter_widget: filter_widget::FilterWidget,
}

impl App {
    pub fn new(providers: Vec<Box<dyn provider::Provider>>) -> Self {
        Self {
            should_exit: false,
            reload_projects: true,
            reload_tasks: true,
            current_block: AppBlock::Providers,
            providers: SelectableList::new(providers, Some(0)),
            projects: SelectableList::default(),
            tasks: SelectableList::default(),
            filter_widget: filter_widget::FilterWidget::new(filter::Filter {
                states: vec![filter::FilterState::Uncompleted],
                due: vec![filter::Due::Today, filter::Due::Overdue],
            }),
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_exit {
            if self.reload_projects {
                self.load_projects().await;
                self.reload_projects = false
            }

            if self.reload_tasks {
                self.load_tasks().await;
                self.reload_tasks = false;
            }

            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    async fn load_projects(&mut self) {
        let mut projects: Vec<Box<dyn project::Project>> = Vec::new();
        let is_all = self.providers.state.selected().unwrap_or_default() == 0;

        for (i, p) in self.providers.items.iter_mut().enumerate() {
            // -1 because of All
            if !is_all && i != self.providers.state.selected().unwrap_or_default() - 1 {
                continue;
            }

            let result = p.projects().await;
            if let Ok(mut prj) = result {
                projects.append(&mut prj);
            }
        }

        self.projects.items = projects;
        self.projects.state = ListState::default().with_selected(Some(0));
    }

    async fn load_tasks(&mut self) {
        let mut tasks: Vec<Box<dyn task::Task>> = Vec::new();
        let is_all = self.providers.state.selected().unwrap_or_default() == 0;

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

            // -1 because of All
            if !is_all && i != self.providers.state.selected().unwrap_or_default() - 1 {
                continue;
            }

            let result = p.tasks(project, &self.filter_widget.filter()).await;
            if let Ok(mut prj) = result {
                tasks.append(&mut prj);
            }
        }

        tasks.sort_by_key(|k| k.due());
        self.tasks.items = tasks;
        self.tasks.state = ListState::default().with_selected(Some(0));
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('h') | KeyCode::Left => self.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.select_last(),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                self.toggle_status();
            }
            KeyCode::Tab => self.select_next_block(),
            KeyCode::Char(' ') => self.change_check_state(),
            _ => {}
        }
    }

    fn change_check_state(&mut self) {
        match self.current_block {
            AppBlock::Providers | AppBlock::Projects | AppBlock::TaskList => {} //TODO: implement
            AppBlock::Filter => {
                self.filter_widget.change_check_state();
                self.reload_tasks = true;
            }
            AppBlock::TaskDescription => { /*nothing to do here */ }
        }
    }

    fn select_next_block(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.current_block = AppBlock::Projects,
            AppBlock::Projects => {
                self.current_block = AppBlock::Filter;
                self.filter_widget.set_active(true);
            }
            AppBlock::Filter => {
                if !self.filter_widget.next_block() {
                    self.current_block = AppBlock::TaskList;
                    self.filter_widget.set_active(false);
                }
            }
            AppBlock::TaskList => self.current_block = AppBlock::TaskDescription,
            AppBlock::TaskDescription => self.current_block = AppBlock::Providers,
        }
    }

    fn set_reload(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.reload_projects = true;
                self.reload_tasks = true;
            }
            AppBlock::Projects => {
                self.reload_tasks = true;
            }
            AppBlock::Filter => {
                self.reload_tasks = true;
            }
            AppBlock::TaskList => {}
            AppBlock::TaskDescription => {}
        }
    }

    fn select_none(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select(None),
            AppBlock::Projects => self.projects.state.select(None),
            AppBlock::Filter => self.filter_widget.select_none(),
            AppBlock::TaskList => self.tasks.state.select(None),
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_next(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select_next(),
            AppBlock::Projects => self.projects.state.select_next(),
            AppBlock::Filter => self.filter_widget.select_next(),
            AppBlock::TaskList => self.tasks.state.select_next(),
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_previous(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.state.select_previous(),
            AppBlock::Projects => self.projects.state.select_previous(),
            AppBlock::Filter => self.filter_widget.select_previous(),
            AppBlock::TaskList => self.tasks.state.select_previous(),
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

    /// Changes the status of the selected list item
    fn toggle_status(&mut self) {
        // if let Some(i) = self.tasks.state.selected() {
        // self.tasks.items[i].status = match self.tasks.items[i].status {
        //     Status::Completed => Status::Todo,
        //     Status::Todo => Status::Completed,
        // }
        // }
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
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(3)]).areas(main_area);

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
    }
}

/// Rendering logic for the app
impl App {
    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Task Aggregator TUI")
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
            return ACTIVE_BLOCK_STYLE;
        }

        INACTIVE_BLOCK_STYLE
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
            .bg(NORMAL_ROW_BG);

        List::new(items.to_vec())
            .block(block)
            .highlight_style(SELECTED_STYLE)
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
            .bg(NORMAL_ROW_BG)
            .render(header_area, buf);
        self.filter_widget.render(area, buf);
    }

    fn render_tasks(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .tasks
            .items
            .iter()
            .map(|t| {
                let mixed_line = Line::from(vec![
                    Span::styled(
                        format!("- [{}] {} (", t.state(), t.text()),
                        Style::default(),
                    ),
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
            self.prepare_render_list("Tasks", AppBlock::TaskList, &items),
            area,
            buf,
            &mut self.tasks.state,
        );
    }

    fn render_task_description(&self, area: Rect, buf: &mut Buffer) {
        // We get the info depending on the item's state.
        let info = if let Some(i) = self.tasks.state.selected() {
            self.tasks.items[i].text()
            // match self.tasks.items[i].status {
            //     Status::Completed => format!("✓ DONE: {}", self.tasks.items[i].info),
            //     Status::Todo => format!("☐ TODO: {}", self.tasks.items[i].info),
            // }
        } else {
            "Nothing selected...".to_string()
        };

        // We show the list item's info under the list in this paragraph
        let block = Block::new()
            .title(Line::raw("TODO Info").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(self.block_style(AppBlock::TaskDescription))
            .bg(NORMAL_ROW_BG)
            .padding(Padding::horizontal(1));

        // We can now render the item info
        Paragraph::new(info)
            .block(block)
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
