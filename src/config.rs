use directories::ProjectDirs;
use ratatui::style::{palette::tailwind, Color};
use serde::Deserialize;
use std::{
    env,
    fs::{self, read_to_string},
};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub file_path: String,
    pub add_creation_date: bool,
    pub move_done_to_end: bool,
    pub searches_path: String,
    pub theme: Theme,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Theme {
    #[serde(with = "color_to_tui")]
    pub completed_text: Color,
    #[serde(with = "color_to_tui")]
    pub context: Color,
    #[serde(with = "color_to_tui")]
    pub priority: Color,
    #[serde(with = "color_to_tui")]
    pub project: Color,
    #[serde(with = "color_to_tui")]
    pub selected: Color,
    #[serde(with = "color_to_tui")]
    pub text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            selected: tailwind::BLUE.c300,
            text: tailwind::SLATE.c200,
            completed_text: tailwind::GRAY.c500,
            context: tailwind::GREEN.c500,
            project: tailwind::AMBER.c500,
            priority: tailwind::EMERALD.c500,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: "".to_string(),
            searches_path: "".to_string(),
            move_done_to_end: true,
            add_creation_date: true,
            theme: Theme::default(),
        }
    }
}

pub fn get_config() -> Config {
    let mut config = match ProjectDirs::from("", "ff2400t", "rtodo") {
        Some(path) => {
            let mut path = path.config_dir().to_path_buf();
            path.push("config.toml");
            match read_to_string(&path) {
                Ok(string) => match toml::from_str::<Config>(&string) {
                    Ok(mut config) => {
                        if config.searches_path.is_empty() {
                            path.pop();
                            path.push("searches.txt");
                            config.searches_path = path.to_string_lossy().to_string();
                        }
                        config
                    }
                    _ => Config::default(),
                },
                Err(_) => Config::default(),
            }
        }
        None => Config::default(),
    };

    if let Ok(args) = parse_args() {
        if let Some(path) = args.config {
            if let Ok(config_path) = fs::canonicalize(path) {
                if let Ok(string) = read_to_string(&config_path) {
                    if let Ok(mut config_arg) = toml::from_str::<Config>(&string) {
                        config_arg.searches_path = config.searches_path;
                        config = config_arg;
                    };
                };
            }
        };
        if let Some(file_path) = args.file {
            if let Ok(path) = fs::canonicalize(file_path) {
                config.file_path = path.to_string_lossy().to_string()
            }
        };
    };

    if config.file_path.is_empty() {
        let mut pwd = env::current_dir().expect("Failed to find the Present working directory");
        pwd.push("todo.txt");
        let file_name = pwd.as_path();
        config.file_path = file_name.to_string_lossy().to_string()
    }

    config
}

#[derive(Debug)]
struct Args {
    file: Option<String>,
    config: Option<String>,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut file = None;
    let mut config = None;
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('c') | Long("config") => {
                config = Some(parser.value()?.string()?);
            }
            Value(val) => {
                file = Some(val.string()?);
            }
            Long("help") => {
                println!("Usage: rtodo [-c|--config=config-file-path] [todo.txt]");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args { file, config })
}
