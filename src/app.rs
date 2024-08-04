use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    style::{palette::tailwind, Color, Style, Styled},
    widgets::{Block, Borders, List, ListItem, ListState, Padding},
    Frame,
};
use std::{fs::write, time::Duration};

use crate::tui::Tui;

const SELECTED_STYLE_FG: Color = tailwind::BLUE.c300;
const TEXT_COLOR: Color = tailwind::SLATE.c200;
const COMPLETED_TEXT_COLOR: Color = tailwind::GREEN.c500;
const DONE_PREFIX: &str = "x ";
const PENDING_PREFIX: &str = "☐ ";

pub struct Model {
    state: ListState,
    tasks: Vec<Task>,
    running_state: RunningState,
}

impl Model {
    pub fn new(tasks: Vec<&str>) -> Self {
        Self {
            state: ListState::default(),
            tasks: tasks.iter().map(|a| Task::new(a)).collect(),
            running_state: RunningState::Running,
        }
    }

    pub fn write(&self, file_name: &str) -> std::io::Result<()> {
        let content = self
            .tasks
            .iter()
            .map(|a| {
                if a.done {
                    a.text.clone()
                } else {
                    a.text.strip_prefix(PENDING_PREFIX).unwrap().to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        write(file_name, content)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum RunningState {
    #[default]
    Running,
    Done,
}

pub enum Message {
    Quit,
    Next,
    Prev,
    ToggleDone,
}

pub fn run_app(terminal: &mut Tui, mut model: &mut Model) -> color_eyre::Result<Option<Message>> {
    model.state.select(Some(0));
    while model.running_state == RunningState::Running {
        terminal.draw(|f| view(&mut model, f))?;

        let mut current_msg = handle_events(model)?;

        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }

    Ok(None)
}

fn view(model: &mut Model, f: &mut Frame<'_>) {
    let block = Block::new()
        .title("Todo List")
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .padding(Padding::uniform(1));
    let list = List::new(
        model
            .tasks
            .iter()
            .map(|a| {
                ListItem::new(a.text.clone()).style(Style::new().set_style(if a.done {
                    COMPLETED_TEXT_COLOR
                } else {
                    TEXT_COLOR
                }))
            })
            .collect::<Vec<ListItem>>(),
    )
    .block(block)
    .highlight_style(SELECTED_STYLE_FG);

    f.render_stateful_widget(list, f.size(), &mut model.state);
}

fn handle_events(_: &mut Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

fn handle_key(key_event: KeyEvent) -> Option<Message> {
    match key_event.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Message::Prev),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::Next),
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Char('d') => Some(Message::ToggleDone),
        _ => None,
    }
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.running_state = RunningState::Done;
            None
        }
        Message::Next => {
            model.state.select_next();
            None
        }
        Message::Prev => {
            model.state.select_previous();
            None
        }
        Message::ToggleDone => {
            if let Some(index) = model.state.selected() {
                if let Some(item) = model.tasks.get_mut(index) {
                    item.toggle_done();
                }
            };
            None
        }
    }
}

#[derive(Clone)]
struct Task {
    text: String,
    done: bool,
}

impl Task {
    fn new(text: &str) -> Self {
        let done = text.starts_with("x ");
        let text = if done {
            text.to_string()
        } else {
            (PENDING_PREFIX.to_owned() + text).to_string()
        };
        Self { done, text }
    }

    fn toggle_done(&mut self) {
        if self.done {
            self.done = false;
            self.text = self.text.replace(DONE_PREFIX, PENDING_PREFIX);
        } else {
            self.done = true;
            self.text = self.text.replace(PENDING_PREFIX, DONE_PREFIX);
        }
    }
}
