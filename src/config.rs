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
    #[serde(with = "color_to_tui")]
    pub selected_text: Color,
    #[serde(with = "color_to_tui")]
    pub text_color: Color,
    #[serde(with = "color_to_tui")]
    pub completed_text_color: Color,
    #[serde(with = "color_to_tui")]
    pub context_color: Color,
    #[serde(with = "color_to_tui")]
    pub project_color: Color,
    pub searches_path: String,
}

const SELECTED_STYLE_FG: Color = tailwind::BLUE.c300;
const TEXT_COLOR: Color = tailwind::SLATE.c200;
const COMPLETED_TEXT_COLOR: Color = tailwind::GRAY.c500;
const CONTEXT_COLOR: Color = tailwind::GREEN.c500;
const PROJECT_COLOR: Color = tailwind::AMBER.c500;

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: "".to_string(),
            searches_path: "".to_string(),
            move_done_to_end: true,
            add_creation_date: true,
            selected_text: SELECTED_STYLE_FG,
            text_color: TEXT_COLOR,
            completed_text_color: COMPLETED_TEXT_COLOR,
            context_color: CONTEXT_COLOR,
            project_color: PROJECT_COLOR,
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
        println!("{:?}", args);
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
