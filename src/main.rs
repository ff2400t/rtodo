use std::{env, fs::read_to_string, str::FromStr};

use app::{run_app, Model};

mod app;
mod errors;
mod tui;
mod ui;

fn main() -> color_eyre::Result<()> {
    let mut pwd = env::current_dir().expect("Failed to find the Present working directory");
    pwd.push("todo.txt");
    let file_name = pwd.as_path();

    let binding = match read_to_string(file_name) {
        Ok(str) => str,
        Err(_) => {
            println!("Failed to find the todo.txt file in the current directory");
            return Ok(());
        }
    };
    let tasks = binding.lines().collect();

    let file_name = String::from(file_name.to_string_lossy());
    let mut model = Model::new(tasks, file_name);
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    let _ = run_app(&mut terminal, &mut model);

    let result = model.write();

    tui::restore()?;
    match result {
        Err(_) => println!("There was an error in saving the todo.txt file"),
        _ => (),
    }

    Ok(())
}
