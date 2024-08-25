use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Styled, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};
use tui_input::Input;

use crate::app::{AppState, Autocomplete, InputState, Model};

pub fn view(model: &mut Model, f: &mut Frame<'_>) {
    let outer_block = Block::new()
        .title_alignment(ratatui::layout::Alignment::Center)
        .padding(Padding::uniform(1));

    let inner_block = outer_block.inner(f.size());

    let chunks = Layout::default()
        .constraints([Constraint::Max(1), Constraint::Min(8), Constraint::Max(1)])
        .split(inner_block);

    render_task_list(&chunks, f, model);
    render_statusline(f, &chunks);
    // Render this last so that Autocomplete rendering works:w:w
    render_search_input(model, &chunks, f);

    match model.app_state {
        AppState::Edit(ref input_state) => {
            let (layout, cursor_x) = render_input(&chunks, &mut model.input, input_state, f);
            render_autocomplete(&mut model.auto_complete, cursor_x, layout, false, f);
        }
        _ => (),
    };
}

fn render_statusline(f: &mut Frame<'_>, chunks: &std::rc::Rc<[Rect]>) {
    let space_2 = "  ";
    let options = [
        " d: Toggle ",
        space_2,
        " e: Edit ",
        space_2,
        " q: Quit ",
        space_2,
        " s: Save ",
        space_2,
        " /: Search ",
        space_2,
        " D: Delete ",
    ];

    let line = options
        .iter()
        .map(|a| {
            if **a != *space_2 {
                Span::styled(*a, Style::default().on_gray().black())
            } else {
                Span::raw(space_2)
            }
        })
        .collect::<Vec<Span>>();
    f.render_widget(Line::from(line), chunks[2]);
}

fn render_search_input(model: &mut Model, chunks: &std::rc::Rc<[Rect]>, f: &mut Frame<'_>) {
    let layout = chunks[0];
    if model.search.active {
        let input_widget = Paragraph::new(model.search.input.value()).block(Block::new());
        f.render_widget(input_widget, layout);
        let cursor_x = layout.x + model.search.input.visual_cursor() as u16;
        //     // Move one line down, from the border to the input line
        f.set_cursor(cursor_x, layout.y);
        render_autocomplete(&mut model.auto_complete, cursor_x, layout, true, f);
    } else {
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
}

fn render_input(
    chunks: &std::rc::Rc<[Rect]>,
    input: &mut Input,
    input_state: &InputState,
    f: &mut Frame<'_>,
) -> (Rect, u16) {
    let layout = centered_rect(50, 30, chunks[1]);
    let width = layout.width.max(3) - 3;
    let scroll = input.visual_scroll(width as usize);
    let title = match input_state {
        InputState::Edit => "Edit Task",
        InputState::NewTask | InputState::CopyTask => "New Task",
    };
    let input_widget = Paragraph::new(input.value())
        .scroll((0, scroll as u16))
        .block(
            Block::default()
                .title_top(title)
                .borders(Borders::ALL)
                .title("Input"),
        );
    f.render_widget(input_widget, layout);
    //     // Put cursor past the end of the input text
    let cursor_x = layout.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 1;
    //     // Move one line down, from the border to the input line
    let cursor_y = layout.y + 1;
    f.set_cursor(cursor_x, cursor_y);
    (layout, cursor_x)
}

fn render_task_list(chunks: &std::rc::Rc<[Rect]>, f: &mut Frame<'_>, model: &mut Model) {
    let list_block = Block::new().borders(Borders::BOTTOM | Borders::TOP);
    let list = if model.search.input.value().is_empty() {
        &model.tasks
    } else {
        &model.filtered_tasks
    };
    let list_widget = List::new(
        list.iter()
            .map(|a| {
                ListItem::new(a.text.as_str()).style(Style::new().set_style(if a.done {
                    model.config.completed_text_color
                } else {
                    model.config.text_color
                }))
            })
            .collect::<Vec<ListItem>>(),
    )
    .block(list_block)
    .highlight_style(model.config.selected_text);

    f.render_stateful_widget(list_widget, chunks[1], &mut model.list_state);
}

fn render_autocomplete(
    auto_complete: &mut Option<Autocomplete>,
    cursor_x: u16,
    layout: Rect,
    is_search: bool,
    f: &mut Frame<'_>,
) {
    if let Some(auto_complete) = auto_complete {
        if auto_complete.list.len() > 0 {
            let block = Block::new().borders(Borders::NONE);
            let list_widget = List::new(
                auto_complete
                    .list
                    .iter()
                    .map(|a| ListItem::new(a.clone()))
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
