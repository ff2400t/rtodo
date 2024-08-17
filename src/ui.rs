use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Styled, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::{AppState, AutoCompleteKind, InputState, Model};

pub fn view(model: &mut Model, f: &mut Frame<'_>) {
    let outer_block = Block::new()
        .title("Todo")
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 1, 0));

    let chunks = render_task_list(outer_block, f, model);

    match model.app_state {
        AppState::Edit(ref input_state) => {
            let layout = centered_rect(50, 30, chunks[0]);
            let width = layout.width.max(3) - 3;
            let scroll = model.input.visual_scroll(width as usize);
            let title = match input_state {
                InputState::Edit => "Edit Task",
                InputState::NewTask => "New Task",
                InputState::Filter => "Search",
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
            let cursor_x =
                layout.x + ((model.input.visual_cursor()).max(scroll) - scroll) as u16 + 1;
            //     // Move one line down, from the border to the input line
            let cursor_y = layout.y + 1;
            f.set_cursor(cursor_x, cursor_y);
            render_autocomplete(model, cursor_x, layout, f, &chunks);
        }
        _ => {
            let line = "d: Toggle Done; e: Edit; q: Quit; s: Search; D: Delete".to_string()
                + if model.app_state == AppState::Filter {
                    &"; Esc: Discard Filter"
                } else {
                    &""
                };
            f.render_widget(Line::raw(line), chunks[1]);
        }
    }
}

fn render_task_list(
    outer_block: Block<'_>,
    f: &mut Frame<'_>,
    model: &mut Model,
) -> std::rc::Rc<[Rect]> {
    let outer_area = outer_block.inner(f.size());
    let chunks = Layout::default()
        .constraints([Constraint::Min(10), Constraint::Max(1)])
        .split(outer_area);
    f.render_widget(outer_block, f.size());
    let list_block = Block::new().borders(Borders::BOTTOM);
    let list = if model.filter_str.is_some() {
        &model.filtered_tasks
    } else {
        &model.tasks
    };
    let list_widget = List::new(
        list.iter()
            .map(|a| {
                ListItem::new(a.text.clone()).style(Style::new().set_style(if a.done {
                    model.config.completed_text_color
                } else {
                    model.config.text_color
                }))
            })
            .collect::<Vec<ListItem>>(),
    )
    .block(list_block)
    .highlight_style(model.config.selected_text);

    f.render_stateful_widget(list_widget, chunks[0], &mut model.state);
    chunks
}

fn render_autocomplete(
    model: &mut Model,
    cursor_x: u16,
    layout: Rect,
    f: &mut Frame<'_>,
    chunks: &std::rc::Rc<[Rect]>,
) {
    if let Some(ref auto_complete) = model.auto_complete {
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
                cursor_x + 1,
                layout.y + 2,
                20,
                auto_complete.list.len() as u16,
            );
            f.render_widget(Clear, rect);
            f.render_stateful_widget(
                list_widget,
                rect,
                &mut model.auto_complete.as_mut().unwrap().list_state,
            )
        }
        let some = match model.auto_complete {
            Some(ref c) => match c.kind {
                AutoCompleteKind::Project => "Project",
                AutoCompleteKind::Context => "Context",
            },
            None => "None",
        };
        f.render_widget(
            Line::raw("Enter: Save; Esc: Exit Edit Mode; D: Delete; ".to_string() + some),
            chunks[1],
        );
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
