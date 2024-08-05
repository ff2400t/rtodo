use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette::tailwind, Color, Style, Styled},
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};
use std::{fs::write, path::Path, time::Duration};
use tui_input::{backend::crossterm::EventHandler, Input};

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
    input: Option<Input>,
}

impl Model {
    pub fn new(tasks: Vec<&str>) -> Self {
        Self {
            state: ListState::default(),
            tasks: tasks.iter().map(|a| Task::new(a)).collect(),
            running_state: RunningState::Running,
            input: None,
        }
    }

    pub fn write(&self, file_name: &Path) -> std::io::Result<()> {
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
    Edit,
}

pub enum Message {
    Quit,
    Next,
    Prev,
    ToggleDone,
    ToggleEdit,
    EditorKey(KeyEvent),
}

pub fn run_app(terminal: &mut Tui, mut model: &mut Model) -> color_eyre::Result<Option<Message>> {
    model.state.select(Some(0));
    while model.running_state != RunningState::Done {
        terminal.draw(|f| view(&mut model, f))?;

        let mut current_msg = handle_events(model)?;

        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }

    Ok(None)
}

fn view(model: &mut Model, f: &mut Frame<'_>) {
    match model.running_state {
        RunningState::Running => {
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
        RunningState::Edit => {
            let layout = centered_rect(50, 50, f.size());
            let width = layout.width.max(3) - 3;
            let input = model.input.as_mut().unwrap();
            let scroll = input.visual_scroll(width as usize);
            let input = Paragraph::new(input.value())
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input, layout);
            f.set_cursor(
                //     // Put cursor past the end of the input text
                layout.x
                    + ((model.input.as_mut().unwrap().visual_cursor()).max(scroll) - scroll) as u16
                    + 1,
                //     // Move one line down, from the border to the input line
                layout.y + 1,
            )
        }
        RunningState::Done => unreachable!(),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn handle_events(model: &Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(&model, key));
            }
        }
    }
    Ok(None)
}

fn handle_key(model: &Model, key_event: KeyEvent) -> Option<Message> {
    match model.running_state {
        RunningState::Running => match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Prev),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Next),
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('d') => Some(Message::ToggleDone),
            KeyCode::Char('e') => Some(Message::ToggleEdit),
            _ => None,
        },
        RunningState::Edit => match key_event.code {
            KeyCode::Esc => Some(Message::ToggleEdit),
            _ => Some(Message::EditorKey(key_event)),
        },
        RunningState::Done => unreachable!(),
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
        Message::ToggleEdit => {
            model.running_state = if model.running_state == RunningState::Running {
                if let Some(index) = model.state.selected() {
                    if let Some(value) = model.tasks.get(index) {
                        let value = if value.done {
                            value.text.clone()
                        } else {
                            value.text.strip_prefix(PENDING_PREFIX).unwrap().to_string()
                        };
                        model.input = Some(Input::new(value));
                    };
                };
                RunningState::Edit
            } else {
                model.input = None;
                RunningState::Running
            };
            None
        }
        Message::EditorKey(event) => match event.code {
            KeyCode::Char('d') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(index) = model.state.selected() {
                    let value = model.input.as_mut().unwrap().value();
                    model.tasks[index] = Task::new(value);
                    Some(Message::ToggleEdit)
                } else {
                    None
                }
            }
            _ => {
                model
                    .input
                    .as_mut()
                    .unwrap()
                    .handle_event(&Event::Key(event));
                None
            }
        },
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
            (PENDING_PREFIX.to_string() + text).to_string()
        };
        Self { done, text }
    }

    fn toggle_done(&mut self) {
        const PRIORITY_KEY: &str = "Pri:";
        if self.done {
            self.done = false;
            let priority_kv = self
                .text
                .split_whitespace()
                .skip(1)
                .find(|word| word.starts_with(PRIORITY_KEY));
            if let Some(pri) = priority_kv {
                let pri = pri.strip_prefix(PRIORITY_KEY).unwrap();
                let pri = format!("({}) ", pri);
                self.text = self.text.get(0..2).unwrap().to_string()
                    + &pri
                    + &self
                        .text
                        .get(2..)
                        .unwrap()
                        .to_string()
                        .replace(&(" ".to_string() + &priority_kv.unwrap()), "")
            }
            self.text = self.text.replace(DONE_PREFIX, PENDING_PREFIX);
        } else {
            self.done = true;
            let mut words = self.text.split_whitespace().skip(1);
            if let Some(word) = words.next() {
                let word = word.to_string();
                if word.len() == 3 && word.starts_with("(") && word.ends_with(")") {
                    if let Some(pri) = word.get(1..2).clone() {
                        self.text = self.text.replace(&(word.clone() + &" "), "")
                            + &" "
                            + &PRIORITY_KEY.to_string()
                            + &pri;
                    }
                }
            }
            self.text = self.text.replace(PENDING_PREFIX, DONE_PREFIX);
        }
    }
}
