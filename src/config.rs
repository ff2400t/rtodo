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
}

const SELECTED_STYLE_FG: Color = tailwind::BLUE.c300;
const TEXT_COLOR: Color = tailwind::SLATE.c200;
const COMPLETED_TEXT_COLOR: Color = tailwind::GREEN.c500;

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: "".to_string(),
            move_done_to_end: true,
            add_creation_date: true,
            selected_text: SELECTED_STYLE_FG,
            text_color: TEXT_COLOR,
            completed_text_color: COMPLETED_TEXT_COLOR,
        }
    }
}

pub fn get_config() -> Config {
    let mut config = match ProjectDirs::from("", "ff2400t", "rtodo") {
        Some(path) => {
            let mut path = path.config_dir().to_path_buf();
            path.push("config.toml");
            match read_to_string(path) {
                Ok(string) => match toml::from_str::<Config>(&string) {
                    Ok(res) => res,
                    _ => Config::default(),
                },
                Err(_) => Config::default(),
                // read this dir and the get the file location = dir.push("config.toml")
            }
        }
        None => Config::default(),
    };

    let args = std::env::args();
    let argument = args.skip(1).next();
    if let Some(arg) = argument {
        if let Ok(path) = fs::canonicalize(arg) {
            config.file_path = path.to_string_lossy().to_string()
        }
    }

    if config.file_path == "" {
        let mut pwd = env::current_dir().expect("Failed to find the Present working directory");
        pwd.push("todo.txt");
        let file_name = pwd.as_path();
        config.file_path = file_name.to_string_lossy().to_string()
    }

    config
}
