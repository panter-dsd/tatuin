// SPDX-License-Identifier: MIT

mod filter;
mod github;
mod github_issues;
mod gitlab;
mod gitlab_todo;
mod obsidian;
mod patched_task;
mod project;
mod provider;
mod settings;
mod state;
mod task;
mod task_patch;
mod todoist;
mod ui;
mod wizard;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use color_eyre::owo_colors::OwoColorize;
use ratatui::style::Color;
use settings::Settings;
use tokio::sync::RwLock;
use ui::style;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, name("PATH_TO_CONFIG_FILE"), help("/path/to/settings.toml"))]
    settings_file: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Providers {},
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
    AddProvider {},
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
    let cli = Cli::parse();

    let mut cfg = if let Some(p) = cli.settings_file {
        Settings::new(p.as_str())
    } else {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("tatuin");
        let config_path = xdg_dirs
            .place_config_file("settings.toml")
            .expect("cannot create configuration directory");
        Settings::new(config_path.to_str().unwrap())
    };

    let mut providers: Vec<Arc<RwLock<Box<dyn provider::Provider>>>> = Vec::new();

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
        if let Some(v) = config.get("disabled") {
            if v.parse::<bool>() == Ok(true) {
                continue;
            }
        }

        match config.get("type").unwrap().as_str() {
            obsidian::PROVIDER_NAME => {
                let mut path = config.get("path").unwrap().to_string();
                if !path.ends_with('/') {
                    path.push('/');
                }

                providers.push(Arc::new(RwLock::new(Box::new(obsidian::Provider::new(
                    name,
                    path.as_str(),
                    color(),
                )))));
            }
            todoist::PROVIDER_NAME => providers.push(Arc::new(RwLock::new(Box::new(todoist::Provider::new(
                name,
                config.get("api_key").unwrap().as_str(),
                color(),
            ))))),
            gitlab_todo::PROVIDER_NAME => providers.push(Arc::new(RwLock::new(Box::new(gitlab_todo::Provider::new(
                name,
                config.get("base_url").unwrap().as_str(),
                config.get("api_key").unwrap().as_str(),
                color(),
            ))))),
            github_issues::PROVIDER_NAME => {
                providers.push(Arc::new(RwLock::new(Box::new(github_issues::Provider::new(
                    name,
                    config.get("api_key").unwrap().as_str(),
                    config.get("repository").unwrap().as_str(),
                    color(),
                )))))
            }
            _ => println!("Unknown provider configuration for section: {name}"),
        }
    }

    if !providers.is_empty() {
        // providers.sort_by_key(|p| p.name()); // TODO: return it back
    }

    match &cli.command {
        Some(Commands::Providers {}) => {
            println!("Available providers: {}", wizard::AVAILABLE_PROVIDERS.join(", "));
        }
        Some(Commands::Tasks { state, due, provider }) => {
            let f = filter::Filter {
                states: state_to_filter(state),
                due: due_to_filter(due),
            };

            let mut tasks = Vec::new();
            for p in providers {
                if let Some(provider_name) = provider {
                    if p.read().await.name() != *provider_name {
                        continue;
                    }
                }

                tasks.append(&mut p.write().await.tasks(None, &f).await?);
            }
            print_boxed_tasks(&tasks);
        }
        Some(Commands::Projects { provider }) => {
            let mut projects = Vec::new();

            for p in providers {
                if let Some(provider_name) = provider {
                    if p.read().await.name() != *provider_name {
                        continue;
                    }
                }

                projects.append(&mut p.write().await.projects().await?);
            }

            print_projects(&projects);
        }
        Some(Commands::AddProvider {}) => {
            let w = wizard::AddProvider {};
            w.run(&mut cfg)?
        }
        _ => {
            color_eyre::install()?;
            let terminal = ratatui::init();
            let _app_result = ui::App::new(providers, Box::new(cfg)).await.run(terminal).await;
            ratatui::restore();
        }
    };
    Ok(())
}
