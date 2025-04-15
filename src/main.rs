mod obsidian;
mod settings;
mod todoist;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use obsidian::task::State;
use settings::Settings;

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
    },
}

#[derive(Subcommand, Debug)]
enum TodoistCommands {
    Tasks {
        #[arg(short, long)]
        state: Option<Vec<ListState>>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ListState {
    Completed,
    Uncompleted,
    InProgress,
    Unknown,
}

fn state_to_list_state(s: &State) -> ListState {
    match s {
        State::Completed => ListState::Completed,
        State::Uncompleted => ListState::Uncompleted,
        State::InProgress => ListState::InProgress,
        State::Unknown(_) => ListState::Unknown,
    }
}

async fn print_obsidian_task_list(
    cfg: Settings,
    states: Vec<ListState>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Open obsidian in path: {}", cfg.obsidian.path);
    let obs = obsidian::Obsidian::new(cfg.obsidian.path.as_str());
    println!("Supported documents count: {}", obs.count()?);

    let tasks = obs.tasks().await?;
    for t in tasks {
        if states.contains(&state_to_list_state(&t.state)) {
            println!(
                "- [{}] {} ({}:{})",
                t.state,
                t.text,
                t.file_path
                    .strip_prefix(cfg.obsidian.path.as_str())
                    .unwrap_or_default()
                    .green(),
                t.pos.to_string().green(),
            );
        }
    }
    Ok(())
}

async fn print_todoist_task_list(
    cfg: Settings,
    states: Vec<ListState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let td = todoist::Todoist::new(&cfg.todoist.api_key);
    let tasks = td.tasks().await?;
    _ = tasks;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Settings::load("settings.toml")?;

    let cli = Cli::parse();
    match &cli.command {
        Commands::Obsidian { command } => match command {
            ObsidianCommands::Tasks { state } => {
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
                print_obsidian_task_list(cfg, states).await?
            }
        },
        Commands::Todoist { command } => match command {
            TodoistCommands::Tasks { state } => {
                let mut states: Vec<ListState> = Vec::new();
                if let Some(st) = state {
                    for s in st {
                        states.push(*s);
                    }
                }
                print_todoist_task_list(cfg, states).await?
            }
        },
    };
    Ok(())
}
