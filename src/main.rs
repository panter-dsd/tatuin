mod obsidian;
mod settings;
mod task;
mod todoist;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use settings::Settings;
use task::State;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Obsidian {
        #[command(subcommand)]
        command: ObsidianCommands,
    },
    Todoist {
        #[command(subcommand)]
        command: TodoistCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ObsidianCommands {
    Tasks {
        #[arg(short, long)]
        state: Option<Vec<ListState>>,

        #[arg(short, long)]
        today: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TodoistCommands {
    Tasks {
        #[arg(short, long)]
        project: Option<String>,
    },
    Projects {},
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ListState {
    Completed,
    Uncompleted,
    InProgress,
    Unknown,
}

const fn state_to_list_state(s: &State) -> ListState {
    match s {
        State::Completed => ListState::Completed,
        State::Uncompleted => ListState::Uncompleted,
        State::InProgress => ListState::InProgress,
        State::Unknown(_) => ListState::Unknown,
    }
}

fn due_to_str(t: Option<task::DateTimeUtc>) -> String {
    if let Some(d) = t {
        if d.time() == chrono::NaiveTime::default() {
            return d.format("%Y-%m-%d").to_string();
        }

        return d.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    String::from("-")
}

fn print_tasks<T: task::Task>(tasks: &Vec<T>) {
    for t in tasks {
        println!(
            "- [{}] {} ({}) ({})",
            t.state(),
            t.text(),
            format!("due: {}", due_to_str(t.due())).blue(),
            t.place().green()
        );
    }
}

fn filter_task<T: task::Task>(t: &T, states: &[ListState], today: bool) -> bool {
    if !states.contains(&state_to_list_state(&t.state())) {
        return false;
    }

    if today {
        if let Some(d) = t.due() {
            let now = chrono::Utc::now().date_naive();
            if d.date_naive() != now {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn filter_tasks<T: task::Task + Clone>(tasks: &[T], states: &[ListState], today: bool) -> Vec<T> {
    tasks
        .iter()
        .filter(|t| filter_task(*t, states, today))
        .cloned()
        .collect()
}

async fn print_obsidian_task_list(
    cfg: Settings,
    states: Vec<ListState>,
    today: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Open obsidian in path: {}", cfg.obsidian.path);
    let obs = obsidian::Obsidian::new(cfg.obsidian.path.as_str());
    println!("Supported documents count: {}", obs.count()?);

    let tasks = obs.tasks().await?;
    let filtered_tasks = filter_tasks(&tasks, &states, today);
    print_tasks(&filtered_tasks);

    Ok(())
}

async fn print_todoist_task_list(
    cfg: Settings,
    project: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let td = todoist::Todoist::new(&cfg.todoist.api_key);
    let filter = todoist::TaskFilter { project };
    let tasks = td.tasks(filter).await?;
    print_tasks(&tasks);

    Ok(())
}

async fn print_todoist_project_list(cfg: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let td = todoist::Todoist::new(&cfg.todoist.api_key);
    let projects = td.projects().await?;

    for p in projects {
        println!("{}: {}", p.id, p.name);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Settings::load("settings.toml")?;

    let cli = Cli::parse();
    match &cli.command {
        Commands::Obsidian { command } => match command {
            ObsidianCommands::Tasks { state, today } => {
                let mut states: Vec<ListState> = Vec::new();
                match state {
                    Some(st) => {
                        for s in st {
                            states.push(*s);
                        }
                    }
                    None => {
                        states.push(ListState::Uncompleted);
                    }
                }
                print_obsidian_task_list(cfg, states, *today).await?
            }
        },
        Commands::Todoist { command } => match command {
            TodoistCommands::Tasks { project } => {
                let p = project.clone();
                print_todoist_task_list(cfg, p).await?
            }
            TodoistCommands::Projects {} => print_todoist_project_list(cfg).await?,
        },
    };
    Ok(())
}
