// SPDX-License-Identifier: MIT

mod async_jobs;
mod migration;
mod provider;
mod settings;
mod ui;
mod wizard;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tatuin_providers::{caldav, config::Config, github_issues, gitlab_todo, ical, obsidian, tatuin, todoist};

use clap::{Parser, Subcommand};
use color_eyre::owo_colors::OwoColorize;
use crossterm::{event::DisableMouseCapture, execute};
use itertools::Itertools;
use provider::Provider;
use ratatui::style::Color;
use settings::Settings;
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use ui::style;

use tatuin_core::{
    filter, folders, project,
    provider::{ProjectProviderTrait, ProviderTrait, TaskProviderTrait},
    task,
};

use crate::migration::migrate_config;

const APP_NAME: &str = "tatuin";
const CONFIG_FILE_NAME: &str = "settings.toml";
const KEEP_LOG_FILES_COUNT: usize = 5;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, name("PATH_TO_CONFIG_FILE"), help("/path/to/settings.toml"))]
    settings_file: Option<String>,

    #[arg(short, long, name("THEME_NAME"), help("theme name"))]
    theme: Option<String>,
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
    ConfigDir {},
}

fn print_boxed_tasks(tasks: &[Box<dyn task::Task>]) {
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
    let log_path = folders::log_folder(APP_NAME);
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

fn load_theme(theme: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(theme) = theme {
        let file_name = folders::config_folder(APP_NAME).join(format!("{theme}.theme"));
        println!("Try to load theme from the file: {file_name:?}");
        return style::load_theme(&file_name);
    }

    Ok(())
}

fn is_true(v: bool) -> bool {
    v
}

fn load_providers(cfg: &Settings) -> Result<Vec<Provider>, Box<dyn std::error::Error>> {
    let providers_colors = style::provider_colors();
    let mut it = providers_colors.iter();
    let mut color = || -> &Color {
        let it = &mut it;

        if let Some(n) = it.next() {
            n
        } else {
            *it = providers_colors.iter();
            it.next().unwrap()
        }
    };

    let mut providers: Vec<Provider> = Vec::new();

    for (name, config) in &cfg.providers {
        if config
            .get("disabled")
            .is_some_and(|v| v.parse::<bool>().is_ok_and(is_true))
        {
            continue;
        }

        let cfg = Config::new(APP_NAME, name);
        let config_value = |key: &str| -> &str { config.get(key).unwrap() };

        let p: Option<Box<dyn ProviderTrait>> = match config_value("type") {
            tatuin::PROVIDER_NAME => Some(Box::new(tatuin::Provider::new(cfg)?)),
            obsidian::PROVIDER_NAME => {
                let mut path = config_value("path").to_string();
                if !path.ends_with('/') {
                    path.push('/');
                }

                Some(Box::new(obsidian::Provider::new(cfg, Path::new(&path))))
            }
            todoist::PROVIDER_NAME => Some(Box::new(todoist::Provider::new(cfg, config_value("api_key")))),
            gitlab_todo::PROVIDER_NAME => Some(Box::new(gitlab_todo::Provider::new(
                cfg,
                config_value("base_url"),
                config_value("api_key"),
            ))),
            github_issues::PROVIDER_NAME => Some(Box::new(github_issues::Provider::new(
                cfg,
                config_value("api_key"),
                config_value("repository"),
            ))),
            ical::PROVIDER_NAME => Some(Box::new(ical::Provider::new(cfg, config_value("url"))?)),
            caldav::PROVIDER_NAME => Some(Box::new(caldav::Provider::new(
                cfg,
                config_value("url"),
                config_value("login"),
                config_value("password"),
            )?)),
            _ => {
                println!("Unknown provider configuration for section: {name}");
                None
            }
        };
        if let Some(p) = p {
            providers.push(provider::Provider {
                name: name.to_string(),
                type_name: p.type_name(),
                color: *color(),
                capabilities: p.capabilities(),
                supported_priorities: p.supported_priorities(),
                provider: Arc::new(RwLock::new(p)),
            });
        }
    }

    providers.sort_by_key(|p| p.name.clone());

    Ok(providers)
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
        let config_dir = folders::config_folder(APP_NAME);
        let config_path = config_dir.join(CONFIG_FILE_NAME);
        if !std::fs::exists(&config_path).is_ok_and(is_true) {
            migrate_config(APP_NAME, CONFIG_FILE_NAME);
        }
        Settings::new(config_path.to_str().unwrap())
    };

    if let Err(e) = load_theme(&cli.theme.or(cfg.theme.clone())) {
        println!("Load theme error: {e}")
    }

    let mut providers = load_providers(&cfg)?;

    if providers.is_empty() {
        println!("There is no provider that has been added yet. Please add one.");
        add_provider(&mut cfg)?;
        providers = load_providers(&cfg)?;
        if providers.is_empty() {
            return Ok(());
        }
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
                if let Some(provider_name) = provider
                    && p.name != *provider_name
                {
                    continue;
                }

                let mut task_provider = p.provider.write().await;
                tasks.append(&mut TaskProviderTrait::list(task_provider.as_mut(), None, &f).await?);
            }
            print_boxed_tasks(&tasks);
        }
        Some(Commands::Projects { provider }) => {
            let mut projects = Vec::new();

            for p in providers {
                if let Some(provider_name) = provider
                    && p.name != *provider_name
                {
                    continue;
                }

                let mut project_provider = p.provider.write().await;
                projects.append(&mut ProjectProviderTrait::list(project_provider.as_mut()).await?);
            }

            print_projects(&projects);
        }
        Some(Commands::AddProvider {}) => add_provider(&mut cfg)?,
        Some(Commands::ConfigDir {}) => println!("{}", folders::config_folder(APP_NAME).to_str().unwrap()),
        _ => {
            tracing::info!("Start tui");
            color_eyre::install()?;
            let _guard = scopeguard::guard((), |_| {
                let _ = execute!(std::io::stdout(), DisableMouseCapture);
                ratatui::restore();
                tracing::info!("End tui");
            });
            let terminal = ratatui::init();
            let app_result = ui::App::new(providers, cfg).await.run(terminal).await;
            if let Err(e) = app_result {
                tracing::error!(target="main", error=?e, "Run app");
                return Err(e.into());
            }
        }
    };

    tracing::info!("End application");
    Ok(())
}
