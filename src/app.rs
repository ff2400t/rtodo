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

pub fn run_app(terminal: &mut crate::tui::Tui, model: &mut Model) -> color_eyre::Result<bool> {
    model.list_state.select(Some(0));
    while model.live_state != LiveState::Done {
        terminal.draw(|f| crate::ui::view(model, f))?;

        let mut current_msg = handle_events(model)?;

        while current_msg.is_some() {
            current_msg = update(model, current_msg.unwrap());
        }
    }

    Ok(model.save_file)
}

#[derive(Debug)]
pub struct Model {
    pub list_state: ListState,
    pub live_state: LiveState,
    pub app_state: AppState,
    pub first_done_index: usize,
    pub tasks: Vec<Task>,
    pub nums: Vec<String>,
    pub filtered_tasks: Vec<Task>,
    pub input: Input,
    pub projects: HashSet<String>,
    pub context: HashSet<String>,
    pub auto_complete: Option<Autocomplete>,
    pub config: Config,
    pub save_file: bool,
    pub search: SearchInput,
    pub saved_searches: SavedSearches,
    pub report: String,
}

impl Model {
    pub fn new(tasks: Vec<&str>, config: Config, saved_searches: Vec<String>) -> Self {
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

        let nums: Vec<String> = (0..tasks.len())
            .map(|e| e.to_string())
            .collect::<Vec<String>>();

        Self {
            live_state: LiveState::Running,
            app_state: AppState::List,
            list_state: ListState::default(),
            tasks,
            nums,
            filtered_tasks: Vec::new(),
            first_done_index,
            search: SearchInput::new(),
            input: Input::default(),
            projects,
            context,
            auto_complete: None,
            config,
            save_file: true,
            saved_searches: SavedSearches::new(saved_searches),
            report: String::from(""),
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

    pub fn new_task(&mut self, value: String) {
        if !value.trim().is_empty() {
            let task = Task::new(&value);
            self.tasks.push(task);
            self.add_to_sets(&value.to_string());
            self.move_done_tasks(self.tasks.len() - 1);
            let new_num = self.nums.len().to_string();
            self.nums.push(new_num);
        }

        if !self.search.is_empty() {
            self.filter_tasks()
        }
    }

    fn update_task(&mut self, only_toggle: bool) {
        let value = self.input.value().to_string();
        let index = if let Some(index) = self.list_state.selected() {
            if self.search.is_empty() {
                index
            } else {
                let text = &self.filtered_tasks[index].text;
                self.tasks
                    .iter()
                    .position(|t| t.text == *text)
                    .unwrap_or_default()
            }
        } else {
            0
        };
        if only_toggle {
            let task = self.tasks[index].toggle_done();
            self.move_done_tasks(index);
            if let Some(new_task_string) = task {
                self.new_task(new_task_string)
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
        enum FilterKind {
            Positive,
            Negative,
        }
        let value = self.search.input.value();
        if value.is_empty() {
            self.filtered_tasks = Vec::new();
        } else {
            let values: Vec<(FilterKind, &str)> = value
                .split(",")
                .map(|e| {
                    if e.starts_with("-") {
                        (FilterKind::Negative, e.strip_prefix('-').unwrap())
                    } else {
                        (FilterKind::Positive, e.trim())
                    }
                })
                .collect();

            self.filtered_tasks = self
                .tasks
                .iter()
                .filter(|t| {
                    values.iter().all(|(kind, string)| {
                        let res = t.text.contains(*string);
                        match kind {
                            FilterKind::Positive => res,
                            FilterKind::Negative => !res,
                        }
                    })
                })
                .cloned()
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
        self.nums.pop();
    }

    fn save_search(&mut self) {
        let value = self.search.input.value();
        self.saved_searches.list.push(value.to_string());

        if !self.config.searches_path.is_empty() {
            let content = self.saved_searches.list.join("\n");
            let path = Path::new(self.config.searches_path.as_str());
            let _ = write(path, content);
        };
    }

    fn gen_report(&mut self) -> String {
        let list = if self.search.is_empty() {
            &self.tasks
        } else {
            &self.filtered_tasks
        };
        let completed = list.iter().filter(|t| t.done).count();
        let total = list.len();
        let todo = total - completed;
        [
            format!("Total tasks:      {total:>5}"),
            format!("Completed Task:   {completed:>5}"),
            format!("Task to do:       {todo:>5}"),
        ]
        .join("\n")
        .to_string()
    }
}

#[derive(Debug)]
pub enum AppState {
    Edit(InputState),
    List,
    SavedSearches,
    SearchInput,
    Report,
    Help,
    Goto(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LiveState {
    Running,
    Done,
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
    SaveSearch,
    OpenSavedSearchesView,
    HandleSavedSearchKeys(KeyEvent),
    HandlePaste(String),
    ToggleReport,
    ToggleHelp,
    GotoStart,
    GotoKeyInput(KeyEvent),
}

fn handle_events(model: &Model) -> color_eyre::Result<Option<Message>> {
    match event::read()? {
        Event::Key(key) => {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(model, key));
            }
        }
        Event::Paste(text) => return Ok(handle_paste(model, text)),
        _ => (),
    }
    Ok(None)
}

fn handle_key(model: &Model, key_event: KeyEvent) -> Option<Message> {
    match model.app_state {
        AppState::List => match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Prev),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Next),
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('Q') => Some(Message::QuitWithoutSave),
            KeyCode::Char('d') => Some(Message::ToggleDone),
            KeyCode::Char('x') => Some(Message::DeleteTask),
            KeyCode::Char('e') => Some(Message::OpenInput(InputState::Edit)),
            KeyCode::Char('n') => Some(Message::OpenInput(InputState::NewTask)),
            KeyCode::Char('c') => Some(Message::OpenInput(InputState::CopyTask)),
            KeyCode::Char('s') => Some(Message::SaveFile),
            KeyCode::Char('a') => Some(Message::SaveSearch),
            KeyCode::Char('l') => Some(Message::OpenSavedSearchesView),
            KeyCode::Char('/') => Some(Message::OpenSearch),
            KeyCode::Char('r') => Some(Message::ToggleReport),
            KeyCode::Char('~') => Some(Message::ToggleHelp),
            KeyCode::Char(':') => Some(Message::GotoStart),
            _ => None,
        },
        AppState::Edit(_) => match key_event.code {
            KeyCode::Esc => Some(Message::DiscardEditor),
            _ => Some(Message::EditorKey(key_event)),
        },
        AppState::SavedSearches => Some(Message::HandleSavedSearchKeys(key_event)),
        AppState::SearchInput => Some(Message::SearchKeyInput(key_event)),
        AppState::Report => Some(Message::ToggleReport),
        AppState::Help => match key_event.code {
            KeyCode::Char('~') | KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleHelp),
            _ => None,
        },
        AppState::Goto(_) => Some(Message::GotoKeyInput(key_event)),
    }
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.live_state = LiveState::Done;
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
            if model.input.value().trim().is_empty() {
                model.delete_selected_task()
            } else {
                match input_state {
                    InputState::Edit => {
                        model.update_task(false);
                    }
                    InputState::NewTask | InputState::CopyTask => {
                        let value = model.input.value().to_string();
                        model.new_task(value);
                    }
                };
            }
            model.app_state = AppState::List;
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
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                model.input = Input::default();
                None
            }
            _ => {
                model.input.handle_event(&Event::Key(event));
                Some(Message::HandleAutoComplete)
            }
        },
        Message::DiscardEditor => {
            model.app_state = AppState::List;
            model.auto_complete = None;
            None
        }
        Message::DeleteTask => {
            model.delete_selected_task();
            None
        }
        Message::HandleAutoComplete => {
            let (val, index) = match model.app_state {
                AppState::Edit(_) => (model.input.value(), model.input.cursor()),
                AppState::SearchInput => (model.search.input.value(), model.search.input.cursor()),
                _ => unreachable!(),
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
                            .cloned()
                            .collect::<Vec<String>>();
                        if !suggestions.is_empty() {
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
                            .cloned()
                            .collect::<Vec<String>>();
                        if !suggestions.is_empty() {
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
            model.live_state = LiveState::Done;
            None
        }
        Message::OpenSearch => {
            model.search.prev_value = model.search.input.value().to_string();
            model.app_state = AppState::SearchInput;
            None
        }
        Message::SearchKeyInput(key_event) => match key_event.code {
            KeyCode::Enter => {
                if model.auto_complete.is_some() {
                    Some(Message::AutoCompleteAppend)
                } else {
                    model.app_state = AppState::List;
                    None
                }
            }
            KeyCode::Esc => {
                model.app_state = AppState::List;
                if !model.search.prev_value.is_empty() {
                    model.search.input = Input::new(model.search.prev_value.clone());
                }
                None
            }
            KeyCode::Tab => Some(Message::AutoCompleteMove(key_event)),
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let value = model.search.input.value();
                model.search.prev_value = value.to_string();
                model.search.input = Input::default();
                None
            }
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
                        let input = match model.app_state {
                            AppState::Edit(_) => &model.input,
                            AppState::SearchInput => &model.search.input,
                            _ => unreachable!(),
                        };
                        let cursor_index = input.cursor();
                        let (before, after) = input.value().split_at(cursor_index);
                        let ac_prefix = match ac.kind {
                            AutoCompleteKind::Project => PROJECT_PREFIX,
                            AutoCompleteKind::Context => CONTEXT_PREFIX,
                        };
                        if let Some((before, _)) = before.rsplit_once(ac_prefix) {
                            let value = before.to_string() + ac_prefix + selected + after;
                            let new_input = Input::new(value)
                                .with_cursor(before.len() + ac_prefix.len() + selected.len());
                            match model.app_state {
                                AppState::Edit(_) => model.input = new_input,
                                AppState::SearchInput => model.search.input = new_input,
                                _ => unreachable!(),
                            };
                        };
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
                    } else if selected == ac.list.len() {
                        ac.list_state.select(Some(0))
                    } else {
                        ac.list_state.select_next()
                    }
                } else {
                    ac.list_state.select(Some(0))
                }
            };
            None
        }
        Message::SaveSearch => {
            model.save_search();
            None
        }
        Message::OpenSavedSearchesView => {
            model.app_state = AppState::SavedSearches;
            model.saved_searches.list_state.select(Some(0));
            None
        }
        Message::HandleSavedSearchKeys(key_event) => {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    model.saved_searches.list_state.select_previous()
                }
                KeyCode::Down | KeyCode::Char('j') => model.saved_searches.list_state.select_next(),
                KeyCode::Enter => {
                    if let Some(index) = model.saved_searches.list_state.selected() {
                        if let Some(text) = model.saved_searches.list.get(index) {
                            model.app_state = AppState::List;
                            model.search.input = Input::new(text.clone());
                            model.filter_tasks();
                        }
                    }
                }
                KeyCode::Esc => {
                    model.app_state = AppState::List;
                    model.save_search();
                }
                KeyCode::Delete => {
                    model.app_state = AppState::List;
                }
                _ => {}
            }
            None
        }
        Message::HandlePaste(text) => {
            let input = match model.app_state {
                AppState::Edit(_) => &model.input,
                AppState::SearchInput => &model.search.input,
                _ => {
                    return None;
                }
            };
            let cursor = input.cursor();
            let (before, after) = input.value().split_at(cursor);
            let new = before.to_string() + &text + after;
            let new_cursor = cursor + text.len();
            let new_input = Input::default().with_value(new).with_cursor(new_cursor);
            match model.app_state {
                AppState::Edit(_) => model.input = new_input,
                AppState::SearchInput => model.search.input = new_input,
                _ => return None,
            };
            None
        }
        Message::ToggleReport => {
            if let AppState::Report = model.app_state {
                model.app_state = AppState::List;
                model.report = "".to_string();
            } else {
                model.app_state = AppState::Report;
                model.report = model.gen_report();
            };
            None
        }
        Message::ToggleHelp => {
            if let AppState::Help = model.app_state {
                model.app_state = AppState::List;
            } else {
                model.app_state = AppState::Help;
            };
            None
        }
        Message::GotoStart => {
            model.app_state = AppState::Goto("".to_string());
            None
        }
        Message::GotoKeyInput(key_event) => {
            match key_event.code {
                KeyCode::Char(d) if d.is_ascii_digit() => {
                    if let AppState::Goto(ref mut str) = model.app_state {
                        str.push(d)
                    };
                }
                KeyCode::Enter => {
                    if let AppState::Goto(ref str) = model.app_state {
                        if let Ok(num) = str.parse::<usize>() {
                            model.list_state.select(Some(num))
                        }
                    }
                    model.app_state = AppState::List;
                }
                KeyCode::Esc => {
                    model.app_state = AppState::List;
                }
                _ => {}
            }
            None
        }
    }
}

fn handle_paste(model: &Model, text: String) -> Option<Message> {
    match model.app_state {
        AppState::SearchInput => Some(Message::HandlePaste(text)),
        _ => None,
    }
}

#[derive(Debug)]
pub struct SearchInput {
    pub input: Input,
    pub prev_value: String,
}

impl SearchInput {
    fn new() -> Self {
        Self {
            input: Input::default(),
            prev_value: "".to_string(),
        }
    }

    fn is_empty(&self) -> bool {
        self.input.value().is_empty()
    }
}

#[derive(Debug)]
pub struct SavedSearches {
    pub list: Vec<String>,
    pub list_state: ListState,
}

impl SavedSearches {
    pub fn new(list: Vec<String>) -> Self {
        Self {
            list,
            list_state: ListState::default().with_selected(Some(0)),
        }
    }
}
