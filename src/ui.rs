use crate::filter;
use crate::task::{Task as TaskTrait, due_group};
use crate::{project, provider, task};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Span;
use ratatui::widgets::{Block, Clear, ListItem, ListState, Paragraph, Widget};
mod filter_widget;
mod header;
mod hyperlink;
mod list;
mod selectable_list;
pub mod style;
mod task_description_widget;
mod tasks_widget;
use selectable_list::SelectableList;

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

pub struct App {
    should_exit: bool,
    reload_tasks: bool,
    providers: SelectableList<Box<dyn provider::Provider>>,
    projects: SelectableList<Box<dyn project::Project>>,
    current_block: AppBlock,

    filter_widget: filter_widget::FilterWidget,
    tasks_widget: tasks_widget::TasksWidget,
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
            projects: SelectableList::new(Vec::new(), None),
            filter_widget: filter_widget::FilterWidget::new(filter::Filter {
                states: vec![filter::FilterState::Uncompleted],
                due: vec![filter::Due::Today, filter::Due::Overdue],
            }),
            tasks_widget: tasks_widget::TasksWidget::default(),
            task_description_widget: task_description_widget::TaskDescriptionWidget::default(),
            alert: None,
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.tasks_widget.set_active(true);

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
        let mut projects = self.tasks_widget.tasks_projects();

        projects.sort_by(|l, r| {
            l.provider()
                .cmp(&r.provider())
                .then_with(|| l.name().cmp(&r.name()))
        });

        self.projects.set_items(projects);
        self.projects
            .set_state(ListState::default().with_selected(Some(0)));
    }

    fn add_error(&mut self, message: &str) {
        self.alert = match self.alert.as_ref() {
            Some(s) => Some(format!("{}\n{}", s, message)),
            None => Some(message.to_string()),
        }
    }

    fn selected_project_id(&self) -> Option<String> {
        if let Some(idx) = self.projects.selected_idx() {
            if idx < 1 || self.projects.is_empty() {
                None
            } else {
                let idx = std::cmp::min(idx, self.projects.len());
                Some(self.projects.item(idx - 1).id())
            }
        } else {
            None
        }
    }

    async fn load_tasks(&mut self) {
        let mut tasks: Vec<Box<dyn task::Task>> = Vec::new();
        let selected_provider_idx = std::cmp::min(
            self.providers.selected_idx().unwrap_or_default(),
            self.providers.len(),
        );

        let mut errors = Vec::new();

        let project_id = self.selected_project_id();

        for (i, p) in self.providers.iter_mut().enumerate() {
            if selected_provider_idx != 0 && i != selected_provider_idx - 1 {
                continue;
            }

            match p.tasks(None, &self.filter_widget.filter()).await {
                Ok(t) => tasks.append(
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

        tasks.sort_by(|l, r| {
            due_group(l.as_ref())
                .cmp(&due_group(r.as_ref()))
                .then_with(|| r.priority().cmp(&l.priority()))
                .then_with(|| l.due().cmp(&r.due()))
        });
        self.tasks_widget.set_tasks(tasks);

        if project_id.is_none() {
            self.load_projects().await;
        }
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
        for p in self.providers.iter_mut() {
            p.reload().await;
        }

        self.reload_tasks = true;
    }

    async fn change_check_state(&mut self) {
        match self.current_block {
            AppBlock::TaskList => {
                let result = self
                    .tasks_widget
                    .change_check_state(&mut self.providers.iter_mut())
                    .await;
                if let Err(e) = result {
                    self.alert = Some(format!("Change state error: {e}"))
                }
                self.reload_tasks = true;
            }
            AppBlock::Filter => {
                self.filter_widget.change_check_state();
                self.projects.select_first();
                self.reload().await;
            }
            _ => {}
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

        self.task_description_widget
            .set_active(self.current_block == AppBlock::TaskDescription);
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

        self.task_description_widget
            .set_active(self.current_block == AppBlock::TaskDescription);
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
        self.task_description_widget
            .set_task(self.tasks_widget.selected_task());
    }

    fn select_next(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.providers.select_next();
                self.projects.select_first();
            }
            AppBlock::Projects => self.projects.select_next(),
            AppBlock::Filter => self.filter_widget.select_next(),
            AppBlock::TaskList => {
                self.tasks_widget.select_next();
                self.set_current_task();
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_previous(&mut self) {
        match self.current_block {
            AppBlock::Providers => {
                self.providers.select_previous();
                self.projects.select_first();
            }
            AppBlock::Projects => self.projects.select_previous(),
            AppBlock::Filter => self.filter_widget.select_previous(),
            AppBlock::TaskList => {
                self.tasks_widget.select_previous();
                self.set_current_task();
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_first(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.select_first(),
            AppBlock::Projects => self.projects.select_first(),
            AppBlock::Filter => self.filter_widget.select_first(),
            AppBlock::TaskList => {
                self.tasks_widget.select_first();
                self.set_current_task();
            }
            AppBlock::TaskDescription => {}
        }
        self.set_reload();
    }

    fn select_last(&mut self) {
        match self.current_block {
            AppBlock::Providers => self.providers.select_last(),
            AppBlock::Projects => self.projects.select_last(),
            AppBlock::Filter => self.filter_widget.select_last(),
            AppBlock::TaskList => {
                self.tasks_widget.select_last();
                self.set_current_task();
            }
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

        let [providers_area, projects_area, filter_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ])
        .areas(left_area);
        let [list_area, task_description_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(20)]).areas(right_area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);
        self.render_providers(providers_area, buf);
        self.render_projects(projects_area, buf);
        self.filter_widget.render(filter_area, buf);
        self.tasks_widget.render(list_area, buf);
        self.task_description_widget
            .render(task_description_area, buf);

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
        let link = hyperlink::Hyperlink::new("[Homepage]", "https://github.com/panter-dsd/tatuin");
        link.render(area, buf);
    }

    fn render_providers(&mut self, area: Rect, buf: &mut Buffer) {
        self.providers.render(
            "Providers",
            self.current_block == AppBlock::Providers,
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

    fn render_projects(&mut self, area: Rect, buf: &mut Buffer) {
        let provider_colors: Vec<(String, Color)> = self
            .providers
            .iter()
            .map(|p| (p.name(), p.color()))
            .collect();
        let provider_color =
            |name: &str| provider_colors.iter().find(|(n, _)| n == name).unwrap().1;

        self.projects.render(
            "Projects",
            self.current_block == AppBlock::Projects,
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
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
