// SPDX-License-Identifier: MIT

mod async_jobs;
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
mod time;
mod todoist;
mod types;
mod ui;
mod wizard;
use std::{path::PathBuf, sync::Arc};

use clap::{Parser, Subcommand};
use color_eyre::owo_colors::OwoColorize;
use itertools::Itertools;
use ratatui::style::Color;
use settings::Settings;
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use ui::style;

use crate::provider::ProviderTrait;

const APP_NAME: &str = "tatuin";
const KEEP_LOG_FILES_COUNT: usize = 5;

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

fn clear_old_logs(path: &PathBuf, file_name_pattern: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = std::fs::read_dir(path)?
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|e| e.file_name().to_str().is_some_and(|s| s.starts_with(file_name_pattern)))
        })
        .map(|e| e.as_ref().unwrap().path())
        .sorted()
        .collect::<Vec<PathBuf>>();
    if files.len() <= KEEP_LOG_FILES_COUNT {
        return Ok(());
    }

    files.truncate(files.len() - KEEP_LOG_FILES_COUNT);
    for f in files {
        std::fs::remove_file(f)?;
    }

    Ok(())
}

fn init_logging() {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    let log_path = xdg_dirs
        .create_state_directory("")
        .expect("cannot create state directory");
    let log_file_pattern = format!("{APP_NAME}.log");

    let file_appender = tracing_appender::rolling::daily(&log_path, &log_file_pattern);
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_max_level(Level::DEBUG)
        .init();
    if let Err(e) = clear_old_logs(&log_path, log_file_pattern.as_str()) {
        tracing::error!(target: "main", error=?e, "Clear old files");
    }
}

fn add_provider(cfg: &mut settings::Settings) -> Result<(), Box<dyn std::error::Error>> {
    let w = wizard::AddProvider {};
    w.run(cfg)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // console_subscriber::init();

    init_logging();

    tracing::info!("Start application");

    let cli = Cli::parse();

    let mut cfg = if let Some(p) = cli.settings_file {
        Settings::new(p.as_str())
    } else {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
        let config_path = xdg_dirs
            .place_config_file("settings.toml")
            .expect("cannot create configuration directory");
        Settings::new(config_path.to_str().unwrap())
    };

    let mut providers: Vec<provider::Provider> = Vec::new();

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

        let p: Option<Box<dyn ProviderTrait>> = match config.get("type").unwrap().as_str() {
            obsidian::PROVIDER_NAME => {
                let mut path = config.get("path").unwrap().to_string();
                if !path.ends_with('/') {
                    path.push('/');
                }

                Some(Box::new(obsidian::Provider::new(name, path.as_str(), color())))
            }
            todoist::PROVIDER_NAME => Some(Box::new(todoist::Provider::new(
                name,
                config.get("api_key").unwrap().as_str(),
                color(),
            ))),
            gitlab_todo::PROVIDER_NAME => Some(Box::new(gitlab_todo::Provider::new(
                name,
                config.get("base_url").unwrap().as_str(),
                config.get("api_key").unwrap().as_str(),
                color(),
            ))),
            github_issues::PROVIDER_NAME => Some(Box::new(github_issues::Provider::new(
                name,
                config.get("api_key").unwrap().as_str(),
                config.get("repository").unwrap().as_str(),
                color(),
            ))),
            _ => {
                println!("Unknown provider configuration for section: {name}");
                None
            }
        };
        if let Some(p) = p {
            providers.push(provider::Provider {
                name: name.to_string(),
                type_name: p.type_name(),
                color: ProviderTrait::color(p.as_ref()),
                provider: Arc::new(RwLock::new(p)),
            });
        }
    }

    if providers.is_empty() {
        println!("There is no provider that has been added yet. Please add one.");
        return add_provider(&mut cfg);
    }

    if !providers.is_empty() {
        providers.sort_by_key(|p| p.name.clone());
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
                    if p.name != *provider_name {
                        continue;
                    }
                }

                tasks.append(&mut p.provider.write().await.tasks(None, &f).await?);
            }
            print_boxed_tasks(&tasks);
        }
        Some(Commands::Projects { provider }) => {
            let mut projects = Vec::new();

            for p in providers {
                if let Some(provider_name) = provider {
                    if p.name != *provider_name {
                        continue;
                    }
                }

                projects.append(&mut p.provider.write().await.projects().await?);
            }

            print_projects(&projects);
        }
        Some(Commands::AddProvider {}) => add_provider(&mut cfg)?,
        _ => {
            tracing::info!("Start tui");
            color_eyre::install()?;
            let terminal = ratatui::init();
            let _app_result = ui::App::new(providers, Box::new(cfg)).await.run(terminal).await;
            ratatui::restore();
            tracing::info!("End tui");
        }
    };

    tracing::info!("End application");
    Ok(())
}
