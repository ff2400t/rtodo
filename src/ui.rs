use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Style, Styled, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};
const SPACE_2: &str = "  ";

use crate::{
    app::{AppState, Autocomplete, InputState, Model},
    tasks::TaskStringTag,
};

pub fn view(model: &mut Model, f: &mut Frame<'_>) {
    let outer_block = Block::new().padding(Padding::uniform(1));

    let inner_area = outer_block.inner(f.area());
    let chunks = Layout::default()
        .constraints([Constraint::Max(1), Constraint::Min(8), Constraint::Max(1)])
        .split(inner_area);

    if let AppState::Help = model.app_state {
        render_help_view(f, &chunks);
    } else {
        render_task_list(&chunks, f, model);
        render_saved_searches_list(model, &chunks, f);

        match model.app_state {
            AppState::Edit(_) => render_input(&chunks, model, f),
            AppState::Report => {
                let rect = centered_rect(50, 30, chunks[1]);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title_top("Report")
                    .title_alignment(ratatui::layout::Alignment::Center);
                let para = Paragraph::new(model.report.clone()).block(block);
                f.render_widget(Clear, rect);
                f.render_widget(para, rect);
            }
            _ => {}
        };

        // Render this last so that Autocomplete rendering works:w:w
        if let AppState::SearchInput = model.app_state {
            render_active_search_input(model, f, chunks[0])
        } else {
            render_static_search_input(model, f, chunks[0]);
        }

        // Render this last so that Autocomplete rendering works:w:w
        if let AppState::Goto(ref num) = model.app_state {
            render_goto_statusline(num, f, &chunks)
        } else {
            render_statusline(f, &chunks);
        }
    }
}

fn render_help_view(f: &mut Frame, chunks: &std::rc::Rc<[Rect]>) {
    let help_block = Block::new().borders(Borders::BOTTOM | Borders::TOP);
    let p = Paragraph::new(
        "d or space - Toggle Done for the Task
x - Delete Task
j or 🡣 - Move to next task
k or 🡩 - Move to prev task
n - Start writing a new task
e - Edit the current task
c - Copy this task and open the editor modal
r - open the report window
Ctrl+d - Clear out the current input
/ - start the search input
l - load a search
a - save a search to be reused later
q - quit
Q - quit without saving any changes
s - Save the current state to disk
~ - Help
: - Goto mode similar to vim or helix

Editing
Ctrl + d - Clear out the current text",
    )
    .block(help_block);
    f.render_widget(p, chunks[1]);
    // Status line
    f.render_widget(
        Line::from(Span::styled(
            " ESC: Task View ",
            Style::default().on_gray().black(),
        )),
        chunks[2],
    );
}

fn render_statusline(f: &mut Frame<'_>, chunks: &std::rc::Rc<[Rect]>) {
    let options = [
        " ~: Help ",
        SPACE_2,
        " d: Toggle ",
        SPACE_2,
        " e: Edit ",
        SPACE_2,
        " n: New Task ",
        SPACE_2,
        " q: Quit ",
        SPACE_2,
        " /: Search ",
        SPACE_2,
        " x: Delete ",
        SPACE_2,
        " l: load Search",
        SPACE_2,
        " a: Save Search",
        SPACE_2,
        " r: Report",
    ];

    let line = options
        .iter()
        .map(|a| {
            if **a != *SPACE_2 {
                Span::styled(*a, Style::default().on_gray().black())
            } else {
                Span::raw(SPACE_2)
            }
        })
        .collect::<Vec<Span>>();
    f.render_widget(Line::from(line), chunks[2]);
}

fn render_goto_statusline(num: &String, f: &mut Frame<'_>, chunks: &std::rc::Rc<[Rect]>) {
    let line = Span::from(":".to_string() + num);
    f.render_widget(line, chunks[2]);
}

fn render_static_search_input(model: &mut Model, f: &mut Frame<'_>, layout: Rect) {
    let text = if model.search.input.value().is_empty() {
        "No search is active at the moment"
    } else {
        model.search.input.value()
    };
    let input_widget = Paragraph::new(text)
        .style(Style::default().gray())
        .block(Block::new());
    f.render_widget(input_widget, layout);
}

fn render_active_search_input(model: &mut Model, f: &mut Frame<'_>, layout: Rect) {
    let input_widget = Paragraph::new(model.search.input.value()).block(Block::new());
    f.render_widget(input_widget, layout);
    let cursor_x = layout.x + model.search.input.visual_cursor() as u16;
    //     // Move one line down, from the border to the input line
    f.set_cursor_position(Position::new(cursor_x, layout.y));
    render_autocomplete(&mut model.auto_complete, cursor_x, layout, true, f);
}

fn render_input(chunks: &std::rc::Rc<[Rect]>, model: &mut Model, f: &mut Frame<'_>) {
    let layout = centered_rect(50, 30, chunks[1]);
    let width = layout.width.max(3) - 3;
    let scroll = model.input.visual_scroll(width as usize);
    let title = match model.app_state {
        AppState::Edit(ref state) => match state {
            InputState::Edit => "Edit Task",
            InputState::NewTask | InputState::CopyTask => "New Task",
        },
        _ => unreachable!(),
    };
    let input_widget = Paragraph::new(model.input.value())
        .scroll((0, scroll as u16))
        .block(
            Block::default()
                .title_top(title)
                .borders(Borders::ALL)
                .title("Input"),
        );
    f.render_widget(input_widget, layout);
    //     // Put cursor past the end of the input text
    let cursor_x = layout.x + ((model.input.visual_cursor()).max(scroll) - scroll) as u16 + 1;
    //     // Move one line down, from the border to the input line
    let cursor_y = layout.y + 1;
    f.set_cursor_position(Position::new(cursor_x, cursor_y));
    render_autocomplete(&mut model.auto_complete, cursor_x, layout, false, f);
}

fn render_task_list(chunks: &std::rc::Rc<[Rect]>, f: &mut Frame<'_>, model: &mut Model) {
    let list_block = Block::new().borders(Borders::BOTTOM | Borders::TOP);
    let layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Max(4), Constraint::Min(10)],
    )
    .split(chunks[1]);
    let (list, nums) = if model.search.input.value().is_empty() {
        (&model.tasks, model.nums.as_slice())
    } else {
        (
            &model.filtered_tasks,
            &model.nums[0..model.filtered_tasks.len()],
        )
    };

    let nums_widget = List::new(nums.iter().map(|a| ListItem::from(Text::raw(a))))
        .block(list_block.clone())
        .highlight_style(model.config.theme.selected);
    let list_widget = List::new(list.iter().map(|a| {
        ListItem::from(Line::from(
            a.arr
                .iter()
                .map(|a| {
                    let color = match a.0 {
                        TaskStringTag::Other => model.config.theme.text,
                        TaskStringTag::Context => model.config.theme.context,
                        TaskStringTag::Project => model.config.theme.project,
                        TaskStringTag::Priority => model.config.theme.priority,
                    };
                    Span::styled(a.1.as_str(), Style::new().set_style(color))
                })
                .collect::<Vec<Span>>(),
        ))
    }))
    .block(list_block)
    .highlight_style(model.config.theme.selected);

    f.render_stateful_widget(nums_widget, layout[0], &mut model.list_state);
    f.render_stateful_widget(list_widget, layout[1], &mut model.list_state);
}

fn render_autocomplete(
    auto_complete: &mut Option<Autocomplete>,
    cursor_x: u16,
    layout: Rect,
    is_search: bool,
    f: &mut Frame<'_>,
) {
    if let Some(auto_complete) = auto_complete {
        if !auto_complete.list.is_empty() {
            let block = Block::new().borders(Borders::NONE);
            let list_widget = List::new(
                auto_complete
                    .list
                    .iter()
                    .cloned()
                    .map(ListItem::new)
                    .collect::<Vec<ListItem>>(),
            )
            .highlight_style(Style::default().white().on_black())
            .style(Style::default().on_white().black())
            .block(block);

            let rect = Rect::new(
                cursor_x + if is_search { 0 } else { 1 },
                layout.y + if is_search { 1 } else { 2 },
                20,
                auto_complete.list.len() as u16,
            );
            f.render_widget(Clear, rect);
            f.render_stateful_widget(list_widget, rect, &mut auto_complete.list_state)
        }
    }
}

fn render_saved_searches_list(model: &mut Model, chunks: &std::rc::Rc<[Rect]>, f: &mut Frame<'_>) {
    if let AppState::SavedSearches = model.app_state {
        let rect = centered_rect(50, 50, chunks[1]);
        let list_block = Block::bordered();
        let list = model
            .saved_searches
            .list
            .iter()
            .map(|a| ListItem::from(Line::raw(a)))
            .collect::<List>()
            .block(list_block)
            .highlight_style(model.config.theme.selected);
        f.render_widget(Clear, rect);
        f.render_stateful_widget(list, rect, &mut model.saved_searches.list_state)
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
