mod filter;
mod obsidian;
mod project;
mod provider;
mod settings;
mod task;
mod todoist;
mod ui;
use clap::{Parser, Subcommand};
use color_eyre::owo_colors::OwoColorize;
use ratatui::style::Color;
use settings::Settings;
use ui::style;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Tui {},
    Tasks {
        #[arg(short, long)]
        state: Option<Vec<filter::FilterState>>,

        #[arg(short, long)]
        due: Option<Vec<filter::Due>>,

        #[arg(short, long)]
        provider: Option<String>,
    },
    Projects {
        #[arg(short, long)]
        provider: Option<String>,
    },
}

fn print_boxed_tasks(tasks: &[Box<dyn task::Task>]) {
    // Rewrite the loop with map/filter AI!
    for t in tasks {
        println!("{}", task::format(t.as_ref()));
    }
}

fn print_projects(projects: &[Box<dyn project::Project>]) {
    for p in projects {
        println!("{}: {} ({})", p.id(), p.name(), p.provider().purple());
    }
}

fn state_to_filter(state: &Option<Vec<filter::FilterState>>) -> Vec<filter::FilterState> {
    match state {
        Some(st) => st.to_vec(),
        None => vec![filter::FilterState::Uncompleted],
    }
}

fn due_to_filter(due: &Option<Vec<filter::Due>>) -> Vec<filter::Due> {
    match due {
        Some(d) => d.to_vec(),
        None => vec![],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("tatuin")?;
    let config_path = xdg_dirs
        .place_config_file("settings.toml")
        .expect("cannot create configuration directory");
    let cfg = Settings::new(config_path.to_str().unwrap());

    let mut providers: Vec<Box<dyn provider::Provider>> = Vec::new();

    let mut it = style::PROVIDER_COLORS.iter();
    let mut color = || -> &Color {
        let it = &mut it;

        if let Some(n) = it.next() {
            n
        } else {
            *it = style::PROVIDER_COLORS.iter();
            it.next().unwrap()
        }
    };

    for (name, config) in &cfg.providers {
        match config.get("type").unwrap().as_str() {
            obsidian::PROVIDER_NAME => {
                let mut path = config.get("path").unwrap().to_string();
                if !path.ends_with('/') {
                    path.push('/');
                }

                providers.push(Box::new(obsidian::Provider::new(
                    name,
                    path.as_str(),
                    color(),
                )));
            }
            todoist::PROVIDER_NAME => providers.push(Box::new(todoist::Provider::new(
                name,
                config.get("api_key").unwrap().as_str(),
                color(),
            ))),
            _ => panic!("Unknown provider configuration for section: {name}"),
        }
    }

    if !providers.is_empty() {
        providers.sort_by_key(|p| p.name());

        println!(
            "Available providers: {}",
            providers
                .iter()
                .map(|p| p.name())
                .collect::<Vec<String>>()
                .join(", ")
        );
    }

    let cli = Cli::parse();
    match &cli.command {
        Commands::Tasks {
            state,
            due,
            provider,
        } => {
            let f = filter::Filter {
                states: state_to_filter(state),
                due: due_to_filter(due),
            };

            let mut tasks = Vec::new();
            for mut p in providers {
                if let Some(provider_name) = provider {
                    if p.name() != *provider_name {
                        continue;
                    }
                }

                tasks.append(&mut p.tasks(None, &f).await?);
            }
            print_boxed_tasks(&tasks);
        }
        Commands::Projects { provider } => {
            let mut projects = Vec::new();

            for mut p in providers {
                if let Some(provider_name) = provider {
                    if p.name() != *provider_name {
                        continue;
                    }
                }

                projects.append(&mut p.projects().await?);
            }

            print_projects(&projects);
        }
        Commands::Tui {} => {
            color_eyre::install()?;
            let terminal = ratatui::init();
            let _app_result = ui::App::new(providers).run(terminal).await;
            ratatui::restore();
        }
    };
    Ok(())
}
