use crate::obsidian;
use crate::settings;
use crate::todoist;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path;

const AVAILABLE_PROVIDERS: &[&str] = &[obsidian::PROVIDER_NAME, todoist::PROVIDER_NAME];

pub struct AddProvider {}

impl AddProvider {
    pub fn run(&self, cfg: &mut settings::Settings) -> Result<(), Box<dyn std::error::Error>> {
        println!("Available providers:");
        for (i, p) in AVAILABLE_PROVIDERS.iter().enumerate() {
            println!("\t{i}) {p}")
        }
        print!(
            "Please, choose a provider (0..{} or q for quit)> ",
            AVAILABLE_PROVIDERS.len() - 1
        );
        let _ = io::stdout().flush();

        let mut input_line = String::new();

        io::stdin()
            .read_line(&mut input_line)
            .expect("Failed to read line");
        input_line = input_line.trim().to_string();
        if input_line == "q" {
            return Ok(());
        }

        match input_line.parse::<usize>() {
            Ok(idx) => {
                if idx >= AVAILABLE_PROVIDERS.len() {
                    return Err(Box::<dyn std::error::Error>::from("Wrong input"));
                }

                let provider = AVAILABLE_PROVIDERS[idx];
                println!("Add provider {}", provider);

                let mut provider_cfg = match provider {
                    obsidian::PROVIDER_NAME => self.add_obsidian()?,
                    todoist::PROVIDER_NAME => self.add_todoist()?,
                    _ => panic!("Unknown provider {provider}"),
                };
                provider_cfg.insert("type".to_string(), provider.to_string());

                let provider_name = self.get_provider_name()?;
                cfg.add_provider(&provider_name, &provider_cfg)?;
            }
            Err(e) => return Err(Box::<dyn std::error::Error>::from(e)),
        }

        Ok(())
    }

    fn add_obsidian(&self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        print!("Please, provide a path to the vault> ");
        let _ = io::stdout().flush();

        let mut input_line = String::new();

        io::stdin()
            .read_line(&mut input_line)
            .expect("Failed to read line");
        input_line = input_line.trim().to_string();

        let p = path::Path::new(&input_line)
            .join(".obsidian")
            .join("app.json");
        if !p.exists() {
            println!("The path doesn't contain a file .obsidian/app.json");
            return Err(Box::<dyn std::error::Error>::from("Wrong vault path"));
        }

        Ok(HashMap::from([("path".to_string(), input_line)]))
    }

    fn add_todoist(&self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        print!("Please, provide an api key> ");
        let _ = io::stdout().flush();

        let mut input_line = String::new();

        io::stdin()
            .read_line(&mut input_line)
            .expect("Failed to read line");
        input_line = input_line.trim().to_string();

        Ok(HashMap::from([("api_key".to_string(), input_line)]))
    }

    fn get_provider_name(&self) -> Result<String, Box<dyn std::error::Error>> {
        print!("Please, provide the new provider's unique name> ");
        let _ = io::stdout().flush();

        let mut input_line = String::new();

        io::stdin().read_line(&mut input_line)?;
        Ok(input_line.trim().to_string())
    }
}
