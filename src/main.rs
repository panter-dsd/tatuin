mod filter;
mod obsidian;
mod project;
mod settings;
mod task;
mod todoist;
mod ui;
use clap::{Parser, Subcommand};
use colored::Colorize;
use settings::Settings;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Tui {},
    Obsidian {
        #[command(subcommand)]
        command: ObsidianCommands,
    },
    Todoist {
        #[command(subcommand)]
        command: TodoistCommands,
    },
    Tasks {
        #[arg(short, long)]
        state: Option<Vec<filter::FilterState>>,

        #[arg(short, long)]
        today: bool,

        #[arg(short, long)]
        provider: Option<String>,
    },
    Projects {
        #[arg(short, long)]
        provider: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ObsidianCommands {
    Tasks {
        #[arg(short, long)]
        state: Option<Vec<filter::FilterState>>,

        #[arg(short, long)]
        today: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TodoistCommands {
    Tasks {
        #[arg(short, long)]
        project: Option<String>,

        #[arg(short, long)]
        state: Option<Vec<filter::FilterState>>,

        #[arg(short, long)]
        today: bool,
    },
    Projects {},
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

fn print_boxed_tasks(tasks: &[Box<dyn task::Task>]) {
    // Rewrite the loop with map/filter AI!
    for t in tasks {
        println!(
            "- [{}] {} ({}) ({} => {})",
            t.state(),
            t.text(),
            format!("due: {}", due_to_str(t.due())).blue(),
            t.provider().purple(),
            t.place().green()
        );
    }
}

fn print_projects(projects: &[Box<dyn project::Project>]) {
    for p in projects {
        println!("{}: {} ({})", p.id(), p.name(), p.provider().purple());
    }
}

async fn print_obsidian_task_list(
    cfg: Settings,
    f: &filter::Filter,
) -> Result<(), Box<dyn std::error::Error>> {
    let obs = obsidian::Obsidian::new(cfg.obsidian.path.as_str());
    let tasks = obs.tasks(f).await?;
    print_tasks(&tasks);

    Ok(())
}

async fn print_todoist_task_list(
    cfg: Settings,
    project: &Option<String>,
    f: &filter::Filter,
) -> Result<(), Box<dyn std::error::Error>> {
    let td = todoist::Todoist::new(&cfg.todoist.api_key);
    let tasks = td.tasks(project, f).await?;
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

fn state_to_filter(state: &Option<Vec<filter::FilterState>>) -> Vec<filter::FilterState> {
    match state {
        Some(st) => st.to_vec(),
        None => vec![filter::FilterState::Uncompleted],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Settings::load("settings.toml")?;

    let providers: Vec<Box<dyn task::Provider>> = vec![
        Box::new(obsidian::ObsidianProvider::new(obsidian::Obsidian::new(
            &cfg.obsidian.path,
        ))),
        Box::new(todoist::TodoistProvider::new(todoist::Todoist::new(
            &cfg.todoist.api_key,
        ))),
    ];

    println!(
        "Available providers: {}",
        providers
            .iter()
            .map(|p| p.name())
            .collect::<Vec<String>>()
            .join(", ")
    );

    let cli = Cli::parse();
    match &cli.command {
        Commands::Obsidian { command } => match command {
            ObsidianCommands::Tasks { state, today } => {
                print_obsidian_task_list(
                    cfg,
                    &filter::Filter {
                        states: state_to_filter(state),
                        today: *today,
                    },
                )
                .await?
            }
        },
        Commands::Todoist { command } => match command {
            TodoistCommands::Tasks {
                project,
                state,
                today,
            } => {
                print_todoist_task_list(
                    cfg,
                    project,
                    &filter::Filter {
                        states: state_to_filter(state),
                        today: *today,
                    },
                )
                .await?
            }
            TodoistCommands::Projects {} => print_todoist_project_list(cfg).await?,
        },
        Commands::Tasks {
            state,
            today,
            provider,
        } => {
            let f = filter::Filter {
                states: state_to_filter(state),
                today: *today,
            };

            let mut tasks = Vec::new();
            for p in providers {
                if let Some(provider_name) = provider {
                    if p.name() != *provider_name {
                        continue;
                    }
                }

                tasks.append(&mut p.tasks(&f).await?);
            }
            print_boxed_tasks(&tasks);
        }
        Commands::Projects { provider } => {
            let mut projects = Vec::new();

            for p in providers {
                if let Some(provider_name) = provider {
                    if p.name() != *provider_name {
                        continue;
                    }
                }

                projects.append(&mut p.projects().await?);
            }

            print_projects(&projects);
        }
        Commands::Tui {} => ui::run().await?,
    };
    Ok(())
}
