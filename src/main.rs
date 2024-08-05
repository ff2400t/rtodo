use std::{env, fs::read_to_string};

use app::{run_app, Model};

mod app;
mod errors;
mod tui;
mod ui;

fn main() -> color_eyre::Result<()> {
    let mut pwd = env::current_dir().expect("Failed to find the Present working directory");
    pwd.push("todo.txt");
    let file_name = pwd.as_path();
    let binding = read_to_string(file_name)
        .expect("Failed to find the todo.txt file in the current directory");
    let tasks = binding.lines().collect();

    let mut model = Model::new(tasks);
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    let _ = run_app(&mut terminal, &mut model);

    let result = model.write(file_name);

    tui::restore()?;
    match result {
        Err(_) => println!("There was an error in saving the todo.txt file"),
        _ => (),
    }

    Ok(())
}
