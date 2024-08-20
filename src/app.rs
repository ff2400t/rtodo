use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use std::{collections::HashSet, fs::write, path::Path};
use time::{format_description::BorrowedFormatItem, macros::format_description, OffsetDateTime};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{config::Config, tasks::Task};

const PENDING_PREFIX: &str = "☐ ";
const PROJECT_PREFIX: &str = "+";
const CONTEXT_PREFIX: &str = "@";
const DATE_FORMAT_STR: &[BorrowedFormatItem] = format_description!("[year]-[month]-[day]");

pub fn run_app(terminal: &mut crate::tui::Tui, mut model: &mut Model) -> color_eyre::Result<bool> {
    model.list_state.select(Some(0));
    while model.app_state != AppState::Done {
        terminal.draw(|f| crate::ui::view(&mut model, f))?;

        let mut current_msg = handle_events(model)?;

        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }

    Ok(model.save_file)
}

#[derive(Debug)]
pub struct Model {
    pub list_state: ListState,
    pub first_done_index: usize,
    pub tasks: Vec<Task>,
    pub filtered_tasks: Vec<Task>,
    pub filter_str: Option<String>,
    pub app_state: AppState,
    pub input: Input,
    pub projects: HashSet<String>,
    pub context: HashSet<String>,
    pub auto_complete: Option<Autocomplete>,
    pub config: Config,
    pub save_file: bool,
}

impl Model {
    pub fn new(tasks: Vec<&str>, config: Config) -> Self {
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
        let (tasks, first_done_index) = {
            let tasks: Vec<Task> = tasks
                .iter()
                .filter(|e| {
                    let temp = e.trim();

                    if temp.is_empty() || temp == "x" {
                        false
                    } else {
                        true
                    }
                })
                .map(|a| Task::new(a))
                .collect();
            if config.move_done_to_end {
                let mut todo_task: Vec<Task> = Vec::with_capacity(tasks.len());
                let mut incomplete_tasks = Vec::new();

                for task in tasks {
                    if !task.done {
                        todo_task.push(task)
                    } else {
                        incomplete_tasks.push(task)
                    }
                }
                let first_done_index = todo_task.len();

                todo_task.append(&mut incomplete_tasks);

                (todo_task, first_done_index)
            } else {
                (tasks, usize::MAX)
            }
        };

        Self {
            list_state: ListState::default(),
            tasks,
            first_done_index,
            filtered_tasks: Vec::new(),
            filter_str: None,
            app_state: AppState::Running,
            input: Input::default(),
            projects,
            context,
            auto_complete: None,
            config,
            save_file: true,
        }
    }

    pub fn write(&self) -> std::io::Result<()> {
        if self.save_file {
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
            let path = Path::new(self.config.file_path.as_str());
            write(path, content)
        } else {
            Ok(())
        }
    }

    /// get context and projets from string and add to the sets
    pub fn add_to_sets(&mut self, task: &str) {
        task.split_whitespace().for_each(|t| {
            if t.starts_with(PROJECT_PREFIX) {
                let val = t.strip_prefix(PROJECT_PREFIX).unwrap();
                if !val.is_empty() {
                    self.projects.insert(val.to_string());
                }
            } else if t.starts_with(CONTEXT_PREFIX) {
                let val = t.strip_prefix(CONTEXT_PREFIX).unwrap();
                if !val.is_empty() {
                    self.context.insert(val.to_string());
                }
            }
        });
    }

    pub fn new_task(&mut self) {
        let value = self.input.value();
        let task = Task::new(value);
        self.tasks.push(task);
        value.to_string();
        self.add_to_sets(&value.to_string());
    }

    fn update_task(&mut self, only_toggle: bool) {
        let value = self.input.value().to_string();
        let index = if let Some(index) = self.list_state.selected() {
            if self.filter_str == None {
                index
            } else {
                let text = &self.filtered_tasks[index].text;
                if let Some(index) = self.tasks.iter().position(|t| t.text == *text) {
                    index
                } else {
                    0
                }
            }
        } else {
            0
        };
        if only_toggle {
            self.tasks[index].toggle_done();
        } else {
            let new_task = Task::new(&value);
            self.add_to_sets(&new_task.text);
            self.tasks[index] = new_task;
        }

        if self.config.move_done_to_end {
            if self.tasks[index].done {
                let swap_index = self.first_done_index - 1;
                if swap_index != index {
                    self.tasks.swap(swap_index, index);
                }
                self.first_done_index -= 1
            } else {
                let swap_index = self.first_done_index;
                if swap_index != index {
                    self.tasks.swap(swap_index, index);
                }
                self.first_done_index += 1
            }
        };
        if self.filter_str != None {
            self.filter_tasks()
        }
    }

    fn filter_tasks(&mut self) {
        if self.filter_str.is_some() {
            if let Some(ref value) = self.filter_str {
                self.filtered_tasks = self
                    .tasks
                    .iter()
                    .filter(|t| t.text.contains(value))
                    .map(|a| a.clone())
                    .collect();
                self.app_state = AppState::Filter;
            }
        }
    }

    fn delete_selected_task(&mut self) {
        if self.filter_str == None {
            if let Some(index) = self.list_state.selected() {
                self.tasks.remove(index);
            };
        } else {
            if let Some(index) = self.list_state.selected() {
                let text = &self.filtered_tasks[index].text;
                if let Some(index) = self.tasks.iter().position(|t| t.text == *text) {
                    self.tasks.remove(index);
                };
            }
            self.filter_tasks();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppState {
    Running,
    Done,
    Edit(InputState),
    Filter,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
    OpenInput(InputState),
    InputAction(InputState),
    DiscardEditor,
    ResetFilter,
    DeleteTask,
    HandleAutoComplete,
    SaveFile,
    QuitWithoutSave,
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
            KeyCode::Char('Q') => Some(Message::QuitWithoutSave),
            KeyCode::Char('d') => Some(Message::ToggleDone),
            KeyCode::Char('D') => Some(Message::DeleteTask),
            KeyCode::Char('e') => Some(Message::OpenInput(InputState::Edit)),
            KeyCode::Char('n') => Some(Message::OpenInput(InputState::NewTask)),
            KeyCode::Char('s') => Some(Message::SaveFile),
            KeyCode::Char('/') => Some(Message::OpenInput(InputState::Filter)),
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
            model.list_state.select_next();
            None
        }
        Message::Prev => {
            model.list_state.select_previous();
            None
        }
        Message::ToggleDone => {
            model.update_task(true);
            None
        }
        Message::OpenInput(input_state) => {
            match input_state {
                InputState::Edit => {
                    if let Some(index) = model.list_state.selected() {
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
                    model.app_state = AppState::Edit(input_state);
                }
                InputState::NewTask => {
                    let base = if model.config.add_creation_date {
                        let local = OffsetDateTime::now_local().unwrap();
                        local.format(&DATE_FORMAT_STR).unwrap()
                    } else {
                        "".to_string()
                    };
                    model.input = Input::new(base + " ");
                    model.app_state = AppState::Edit(input_state);
                }
                InputState::Filter => {
                    model.input.reset();
                    model.app_state = AppState::Edit(input_state);
                }
            }
            None
        }
        Message::InputAction(input_state) => {
            match input_state {
                InputState::Edit => {
                    model.update_task(false);
                }
                InputState::NewTask => {
                    model.new_task();
                    model.app_state = AppState::Running;
                }
                InputState::Filter => {
                    let value = model.input.value().to_string();
                    model.filter_str = Some(value);
                    model.filter_tasks();
                }
            };
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
                            let ac_prefix = match ac.kind {
                                AutoCompleteKind::Project => PROJECT_PREFIX,
                                AutoCompleteKind::Context => CONTEXT_PREFIX,
                            };
                            let (before, _) = before.rsplit_once(ac_prefix).unwrap();
                            let value = before.to_string() + ac_prefix + selected + after;
                            model.input = Input::new(value);
                            model.auto_complete = None;
                            return None;
                        }
                    }
                }

                if let AppState::Edit(ref input_state) = model.app_state {
                    Some(Message::InputAction(input_state.clone()))
                } else {
                    None
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
        Message::DiscardEditor => {
            model.app_state = AppState::Running;
            model.auto_complete = None;
            None
        }
        Message::ResetFilter => {
            model.app_state = AppState::Running;
            model.filtered_tasks = Vec::new();
            model.filter_str = None;
            None
        }
        Message::DeleteTask => {
            model.delete_selected_task();
            None
        }
        Message::HandleAutoComplete => {
            let val = model.input.value();
            let index = model.input.cursor();
            if index != 0 {
                let before = val.get(..index).unwrap_or("");
                if before.ends_with(" ") {
                    model.auto_complete = None
                } else if let Some(last_word) = before.split_whitespace().last() {
                    if last_word.starts_with(PROJECT_PREFIX) {
                        let match_word = last_word.strip_prefix(PROJECT_PREFIX).unwrap();
                        let suggestions = model
                            .projects
                            .iter()
                            .filter(|s| s.contains(match_word))
                            .map(|s| s.clone())
                            .collect::<Vec<String>>();
                        model.auto_complete = Some(Autocomplete {
                            kind: AutoCompleteKind::Project,
                            list: suggestions,
                            list_state: ListState::default(),
                        })
                    } else if last_word.starts_with(CONTEXT_PREFIX) {
                        let match_word = last_word.strip_prefix(CONTEXT_PREFIX).unwrap();
                        let suggestions = model
                            .context
                            .iter()
                            .filter(|s| s.contains(match_word))
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
                }
            };
            None
        }
        Message::SaveFile => {
            model.write().ok();
            None
        }
        Message::QuitWithoutSave => {
            model.save_file = false;
            model.app_state = AppState::Done;
            None
        }
    }
}
