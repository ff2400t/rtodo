use std::fs::read_to_string;

use app::{run_app, Model};

mod app;
mod errors;
mod tui;

fn main() -> color_eyre::Result<()> {
    let file_name = "./todo.txt";
    let binding = match read_to_string(file_name) {
        Ok(content) => content,
        Err(_) => {
            eprintln!("Failed to find the todo.txt file in the current directory");
            return Ok(());
        }
    };
    let lines = binding.lines();

    let tasks: _ = lines.map(|a| a).collect();

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
