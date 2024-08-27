use chrono::Local;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use std::{collections::HashSet, fs::write, path::Path};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    config::Config,
    tasks::{Task, DATE_FORMAT_CONST},
};

const PENDING_PREFIX: &str = "☐ ";
const PROJECT_PREFIX: &str = "+";
const CONTEXT_PREFIX: &str = "@";

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
    pub search: SearchInput,
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

                    !(temp.is_empty() || temp == "x")
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
            search: SearchInput::new(),
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
        self.add_to_sets(&value.to_string());
        self.move_done_tasks(self.tasks.len() - 1);
    }

    fn update_task(&mut self, only_toggle: bool) {
        let value = self.input.value().to_string();
        let index = if let Some(index) = self.list_state.selected() {
            if self.search.is_empty() {
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
            let task = self.tasks[index].toggle_done();
            self.move_done_tasks(index);
            if let Some(new_task) = task {
                self.add_to_sets(&new_task.text);
                self.tasks.push(new_task);
                let index = self.tasks.len() - 1;
                self.move_done_tasks(index);
            }
        } else {
            let new_task = Task::new(&value);
            let move_task = self.tasks[index].done != new_task.done;
            self.add_to_sets(&new_task.text);
            self.tasks[index] = new_task;
            if move_task {
                self.move_done_tasks(index);
            }
        };

        if !self.search.is_empty() {
            self.filter_tasks()
        }
    }

    fn move_done_tasks(&mut self, index: usize) {
        if self.config.move_done_to_end {
            if self.tasks[index].done {
                // a new task which is marked as done shouldn't be moved
                if index < self.first_done_index && self.first_done_index != index {
                    self.tasks[index..self.first_done_index].rotate_left(1);
                }
                self.first_done_index -= 1
            } else {
                if self.first_done_index != index {
                    self.tasks[self.first_done_index..index + 1].rotate_right(1);
                }
                self.first_done_index += 1
            }
        };
    }

    fn filter_tasks(&mut self) {
        let value = self.search.input.value();
        if value.is_empty() {
            self.filtered_tasks = Vec::new();
        } else {
            self.filtered_tasks = self
                .tasks
                .iter()
                .filter(|t| t.text.contains(value))
                .map(|a| a.clone())
                .collect();
        }
    }

    fn delete_selected_task(&mut self) {
        if self.search.is_empty() {
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
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InputState {
    Edit,
    NewTask,
    CopyTask,
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

#[derive(Clone)]
pub enum Message {
    Quit,
    Next,
    Prev,
    ToggleDone,
    EditorKey(KeyEvent),
    SearchKeyInput(KeyEvent),
    OpenInput(InputState),
    InputAction(InputState),
    OpenSearch,
    DiscardEditor,
    DeleteTask,
    HandleAutoComplete,
    AutoCompleteAppend,
    AutoCompleteMove(KeyEvent),
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
        AppState::Running if model.search.active => Some(Message::SearchKeyInput(key_event)),
        AppState::Running => match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Prev),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Next),
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('Q') => Some(Message::QuitWithoutSave),
            KeyCode::Char('d') => Some(Message::ToggleDone),
            KeyCode::Char('D') => Some(Message::DeleteTask),
            KeyCode::Char('e') => Some(Message::OpenInput(InputState::Edit)),
            KeyCode::Char('n') => Some(Message::OpenInput(InputState::NewTask)),
            KeyCode::Char('c') => Some(Message::OpenInput(InputState::CopyTask)),
            KeyCode::Char('s') => Some(Message::SaveFile),
            KeyCode::Char('/') => Some(Message::OpenSearch),
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
                        let list = if !model.search.is_empty() {
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
                        let local = Local::now();
                        local.format_with_items(DATE_FORMAT_CONST).to_string()
                    } else {
                        "".to_string()
                    };
                    model.input = Input::new(base + " ");
                    model.app_state = AppState::Edit(input_state);
                }
                InputState::CopyTask => {
                    if let Some(index) = model.list_state.selected() {
                        let list = if !model.search.is_empty() {
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
            }
            None
        }
        Message::InputAction(input_state) => {
            match input_state {
                InputState::Edit => {
                    model.update_task(false);
                }
                InputState::NewTask | InputState::CopyTask => {
                    model.new_task();
                }
            };
            model.app_state = AppState::Running;
            None
        }
        Message::EditorKey(event) => match event.code {
            KeyCode::Enter => {
                if model.auto_complete.is_some() {
                    Some(Message::AutoCompleteAppend)
                } else if let AppState::Edit(ref input_state) = model.app_state {
                    Some(Message::InputAction(input_state.clone()))
                } else {
                    None
                }
            }
            KeyCode::Tab => Some(Message::AutoCompleteMove(event)),
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
        Message::DeleteTask => {
            model.delete_selected_task();
            None
        }
        Message::HandleAutoComplete => {
            let (val, index) = if model.search.active {
                (model.search.input.value(), model.search.input.cursor())
            } else {
                (model.input.value(), model.input.cursor())
            };
            if index != 0 {
                let before = val.get(..index).unwrap_or("");
                if before.is_empty() || before.ends_with(" ") {
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
                        if suggestions.len() > 0 {
                            model.auto_complete = Some(Autocomplete {
                                kind: AutoCompleteKind::Project,
                                list: suggestions,
                                list_state: ListState::default().with_selected(Some(0)),
                            })
                        }
                    } else if last_word.starts_with(CONTEXT_PREFIX) {
                        let match_word = last_word.strip_prefix(CONTEXT_PREFIX).unwrap();
                        let suggestions = model
                            .context
                            .iter()
                            .filter(|s| s.contains(match_word))
                            .map(|s| s.clone())
                            .collect::<Vec<String>>();
                        if suggestions.len() > 0 {
                            model.auto_complete = Some(Autocomplete {
                                kind: AutoCompleteKind::Context,
                                list: suggestions,
                                list_state: ListState::default().with_selected(Some(0)),
                            })
                        }
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
        Message::OpenSearch => {
            model.search.prev_value = model.search.input.value().to_string();
            model.search.active = true;
            None
        }
        Message::SearchKeyInput(key_event) => match key_event.code {
            KeyCode::Enter => {
                if model.auto_complete.is_some() {
                    Some(Message::AutoCompleteAppend)
                } else {
                    model.search.active = false;
                    None
                }
            }
            KeyCode::Esc => {
                model.search.active = false;
                if !model.search.prev_value.is_empty() {
                    model.search.input = Input::new(model.search.prev_value.clone());
                }
                None
            }
            KeyCode::Tab => Some(Message::AutoCompleteMove(key_event)),
            _ => {
                model.search.input.handle_event(&Event::Key(key_event));
                model.filter_tasks();
                Some(Message::HandleAutoComplete)
            }
        },
        Message::AutoCompleteAppend => {
            if let Some(ac) = model.auto_complete.as_ref() {
                if let Some(index) = ac.list_state.selected() {
                    if let Some(selected) = ac.list.get(index) {
                        let input = if model.search.active {
                            &model.search.input
                        } else {
                            &model.input
                        };
                        let cursor_index = input.cursor();
                        let (before, after) = input.value().split_at(cursor_index);
                        let ac_prefix = match ac.kind {
                            AutoCompleteKind::Project => PROJECT_PREFIX,
                            AutoCompleteKind::Context => CONTEXT_PREFIX,
                        };
                        if let Some((before, _)) = before.rsplit_once(ac_prefix) {
                            let value = before.to_string() + ac_prefix + selected + after;
                            if model.search.active {
                                model.search.input = Input::new(value)
                                    .with_cursor(before.len() + ac_prefix.len() + selected.len());
                            } else {
                                model.input = Input::new(value)
                                    .with_cursor(before.len() + ac_prefix.len() + selected.len());
                            };
                        }
                        model.auto_complete = None;
                    };
                };
            }
            None
        }
        Message::AutoCompleteMove(key_event) => {
            if let Some(ref mut ac) = model.auto_complete {
                if let Some(selected) = ac.list_state.selected() {
                    if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                        if selected == 0 {
                            ac.list_state.select(Some(ac.list.len() - 1))
                        } else {
                            ac.list_state.select_previous()
                        }
                        model
                            .auto_complete
                            .as_mut()
                            .unwrap()
                            .list_state
                            .select_previous();
                    } else {
                        if selected == ac.list.len() {
                            ac.list_state.select(Some(0))
                        } else {
                            ac.list_state.select_next()
                        }
                    }
                } else {
                    ac.list_state.select(Some(0))
                }
            };
            None
        }
    }
}

#[derive(Debug)]
pub struct SearchInput {
    pub input: Input,
    pub active: bool,
    pub prev_value: String,
}

impl SearchInput {
    fn new() -> Self {
        Self {
            input: Input::default(),
            active: false,
            prev_value: "".to_string(),
        }
    }

    fn is_empty(&self) -> bool {
        self.input.value().is_empty()
    }
}
