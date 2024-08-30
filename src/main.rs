use std::fs::read_to_string;

use app::{run_app, Model};

mod app;
mod config;
mod errors;
mod tasks;
mod tui;
mod ui;

fn main() -> color_eyre::Result<()> {
    let config = config::get_config();

    let tasks_str = match read_to_string(config.file_path.as_str()) {
        Ok(str) => str,
        Err(_) => {
            println!("Failed to find the todo.txt file");
            return Ok(());
        }
    };
    let tasks = tasks_str.lines().collect();
    let saved_searches = if !config.searches_path.is_empty() {
        match read_to_string(config.searches_path.as_str()) {
            Ok(str) => str.lines().map(|a| a.to_string()).collect(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let mut model = Model::new(tasks, config, saved_searches);
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    let save_file = run_app(&mut terminal, &mut model)?;

    tui::restore()?;

    // this needs to matched after restoring the terminal checked so that the line is printed to the console
    if save_file {
        match model.write() {
            Err(_) => println!("There was an error in saving the todo.txt file"),
            _ => (),
        }
    }

    Ok(())
}
