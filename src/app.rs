use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use std::{collections::HashSet, fs::write, path::Path};
use time::{format_description::BorrowedFormatItem, macros::format_description, OffsetDateTime};
use tui_input::{backend::crossterm::EventHandler, Input};

const DONE_PREFIX: &str = "x ";
const PENDING_PREFIX: &str = "☐ ";
const PROJECT_PREFIX: &str = "+";
const CONTEXT_PREFIX: &str = "@";
const DATE_FORMAT_STR: &[BorrowedFormatItem] = format_description!("[year]-[month]-[day]");

pub fn run_app(
    terminal: &mut crate::tui::Tui,
    mut model: &mut Model,
) -> color_eyre::Result<Option<Message>> {
    model.state.select(Some(0));
    while model.app_state != AppState::Done {
        terminal.draw(|f| crate::ui::view(&mut model, f))?;

        let mut current_msg = handle_events(model)?;

        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }

    Ok(None)
}

#[derive(Debug)]
pub struct Model {
    pub state: ListState,
    pub tasks: Vec<Task>,
    pub filtered_tasks: Vec<Task>,
    pub filter_str: Option<String>,
    pub app_state: AppState,
    pub input: Input,
    pub projects: HashSet<String>,
    pub context: HashSet<String>,
    pub auto_complete: Option<Autocomplete>,
}

impl Model {
    pub fn new(tasks: Vec<&str>) -> Self {
        let mut projects = HashSet::new();
        let mut context = HashSet::new();
        tasks.iter().for_each(|t| {
            t.split_whitespace().for_each(|t| {
                if t.starts_with(PROJECT_PREFIX) {
                    let val = t.strip_prefix(PROJECT_PREFIX).unwrap();
                    projects.insert(val.to_string());
                } else if t.starts_with(CONTEXT_PREFIX) {
                    let val = t.strip_prefix(CONTEXT_PREFIX).unwrap();
                    context.insert(val.to_string());
                } else {
                    ();
                }
            })
        });
        Self {
            state: ListState::default(),
            tasks: tasks.iter().map(|a| Task::new(a)).collect(),
            filtered_tasks: Vec::new(),
            filter_str: None,
            app_state: AppState::Running,
            input: Input::default(),
            projects,
            context,
            auto_complete: None,
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

#[derive(Debug, PartialEq, Eq)]
pub enum AppState {
    Running,
    Done,
    Edit(InputState),
    Filter,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputState {
    Edit,
    NewTask,
    Filter,
}

#[derive(Debug)]
pub struct Autocomplete {
    pub kind: AutoCompleteKind,
    pub list: Vec<String>,
    pub list_state: ListState,
}

#[derive(Debug)]
pub enum AutoCompleteKind {
    Project,
    Context,
}

pub enum Message {
    Quit,
    Next,
    Prev,
    ToggleDone,
    EditorKey(KeyEvent),
    TaskEdit,
    NewTaskEditor,
    SaveNewTask,
    UpdateSelectedTask,
    DiscardEditor,
    FilterEditor,
    FilterList,
    UpdateFilterStr,
    ResetFilter,
    DeleteTask,
    HandleAutoComplete,
}

fn handle_events(model: &Model) -> color_eyre::Result<Option<Message>> {
    if let Event::Key(key) = event::read()? {
        if key.kind == event::KeyEventKind::Press {
            return Ok(handle_key(&model, key));
        }
    }
    Ok(None)
}

fn handle_key(model: &Model, key_event: KeyEvent) -> Option<Message> {
    match model.app_state {
        AppState::Running | AppState::Filter => match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Prev),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Next),
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('d') => Some(Message::ToggleDone),
            KeyCode::Char('D') => Some(Message::DeleteTask),
            KeyCode::Char('e') => Some(Message::TaskEdit),
            KeyCode::Char('n') => Some(Message::NewTaskEditor),
            KeyCode::Char('s') => Some(Message::FilterEditor),
            KeyCode::Esc if model.app_state == AppState::Filter => Some(Message::ResetFilter),
            _ => None,
        },
        AppState::Edit(_) => match key_event.code {
            KeyCode::Esc => Some(Message::DiscardEditor),
            _ => Some(Message::EditorKey(key_event)),
        },
        AppState::Done => unreachable!(),
    }
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.app_state = AppState::Done;
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
        Message::TaskEdit => {
            if let Some(index) = model.state.selected() {
                let list = if model.app_state == AppState::Filter {
                    &model.filtered_tasks
                } else {
                    &model.tasks
                };
                if let Some(value) = list.get(index) {
                    let value = if value.done {
                        value.text.clone()
                    } else {
                        value.text.strip_prefix(PENDING_PREFIX).unwrap().to_string()
                    };
                    model.input = Input::new(value);
                };
            };
            model.app_state = AppState::Edit(InputState::Edit);
            None
        }
        Message::EditorKey(event) => match event.code {
            KeyCode::Enter => {
                if model.auto_complete.is_some() {
                    let ac = model.auto_complete.as_ref().unwrap();
                    if let Some(index) = ac.list_state.selected() {
                        if let Some(selected) = ac.list.get(index) {
                            let cursor_index = model.input.cursor();
                            let (before, after) = model.input.value().split_at(cursor_index);
                            let value = before.to_string() + selected + after;
                            model.input = Input::new(value);
                            model.auto_complete = None;
                            return None;
                        }
                    }
                }

                match model.app_state {
                    AppState::Edit(ref input_state) => match input_state {
                        InputState::Edit => Some(Message::UpdateSelectedTask),
                        InputState::NewTask => Some(Message::SaveNewTask),
                        InputState::Filter => Some(Message::UpdateFilterStr),
                    },
                    _ => unreachable!(),
                }
            }
            KeyCode::Tab => {
                if model.auto_complete.is_some() {
                    if event.modifiers.contains(KeyModifiers::SHIFT) {
                        model
                            .auto_complete
                            .as_mut()
                            .unwrap()
                            .list_state
                            .select_previous();
                    } else {
                        model
                            .auto_complete
                            .as_mut()
                            .unwrap()
                            .list_state
                            .select_next();
                    }
                };
                None
            }
            _ => {
                model.input.handle_event(&Event::Key(event));
                Some(Message::HandleAutoComplete)
            }
        },
        Message::NewTaskEditor => {
            let local = OffsetDateTime::now_local().unwrap();
            let base = local.format(&DATE_FORMAT_STR).unwrap();
            model.input = Input::new(base + " ");
            model.app_state = AppState::Edit(InputState::NewTask);
            None
        }
        Message::SaveNewTask => {
            let value = model.input.value();
            let task = Task::new(value);
            model.tasks.push(task);
            model.app_state = AppState::Running;
            None
        }
        Message::UpdateSelectedTask => {
            if model.filter_str == None {
                if let Some(index) = model.state.selected() {
                    let value = model.input.value();
                    let new_task = Task::new(value);
                    model.tasks[index] = new_task;
                    model.app_state = AppState::Running;
                }
                None
            } else {
                if let Some(index) = model.state.selected() {
                    let value = model.input.value();
                    let text = &model.filtered_tasks[index].text;
                    if let Some(index) = model.tasks.iter().position(|t| t.text == *text) {
                        model.tasks[index] = Task::new(value);
                    }
                }
                Some(Message::FilterList)
            }
        }
        Message::DiscardEditor => {
            model.app_state = AppState::Running;
            model.auto_complete = None;
            None
        }
        Message::FilterEditor => {
            model.input.reset();
            model.app_state = AppState::Edit(InputState::Filter);
            None
        }
        Message::FilterList => {
            if model.filter_str.is_some() {
                if let Some(ref value) = model.filter_str {
                    model.filtered_tasks = model
                        .tasks
                        .iter()
                        .filter(|t| t.text.contains(value))
                        .map(|a| a.clone())
                        .collect();
                    model.app_state = AppState::Filter;
                }
            }
            None
        }
        Message::ResetFilter => {
            model.app_state = AppState::Running;
            model.filtered_tasks = Vec::new();
            model.filter_str = None;
            None
        }
        Message::UpdateFilterStr => {
            let value = model.input.value().to_string();
            model.filter_str = Some(value);
            Some(Message::FilterList)
        }
        Message::DeleteTask => {
            if model.filter_str == None {
                if let Some(index) = model.state.selected() {
                    model.tasks.remove(index);
                };
                None
            } else {
                if let Some(index) = model.state.selected() {
                    let text = &model.filtered_tasks[index].text;
                    if let Some(index) = model.tasks.iter().position(|t| t.text == *text) {
                        model.tasks.remove(index);
                    };
                }
                Some(Message::FilterList)
            }
        }
        Message::HandleAutoComplete => {
            let val = model.input.value();
            let index = model.input.cursor();
            if index != 0 {
                let last_char = val.get(index - 1..index).unwrap_or("");
                if last_char == "+" {
                    let suggestions = model
                        .projects
                        .iter()
                        .map(|s| s.clone())
                        .collect::<Vec<String>>();
                    model.auto_complete = Some(Autocomplete {
                        kind: AutoCompleteKind::Project,
                        list: suggestions,
                        list_state: ListState::default(),
                    })
                } else if last_char == "@" {
                    let suggestions = model
                        .context
                        .iter()
                        .map(|s| s.clone())
                        .collect::<Vec<String>>();
                    model.auto_complete = Some(Autocomplete {
                        kind: AutoCompleteKind::Context,
                        list: suggestions,
                        list_state: ListState::default(),
                    })
                } else {
                    model.auto_complete = None
                }
            };
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub text: String,
    pub done: bool,
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
