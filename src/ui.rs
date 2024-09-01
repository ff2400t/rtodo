use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Style, Styled, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::{
    app::{AppState, Autocomplete, InputState, Model},
    tasks::TaskStringTag,
};

pub fn view(model: &mut Model, f: &mut Frame<'_>) {
    let outer_block = Block::new()
        .title_alignment(ratatui::layout::Alignment::Center)
        .padding(Padding::uniform(1));

    let inner_block = outer_block.inner(f.area());

    let chunks = Layout::default()
        .constraints([Constraint::Max(1), Constraint::Min(8), Constraint::Max(1)])
        .split(inner_block);

    render_task_list(&chunks, f, model);
    render_statusline(f, &chunks);
    render_saved_searches_list(model, &chunks, f);

    match model.app_state {
        AppState::Edit(_) => {
            render_input(&chunks, model, f);
            render_static_search_input(model, f, chunks[0])
        }
        // Render this last so that Autocomplete rendering works:w:w
        AppState::SearchInput => render_active_search_input(model, f, chunks[0]),

        _ => render_static_search_input(model, f, chunks[0]),
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
        space_2,
        " l: load Search",
        space_2,
        " a: Save Search",
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
    let list = if model.search.input.value().is_empty() {
        &model.tasks
    } else {
        &model.filtered_tasks
    };
    let list_widget = List::new(
        list.iter()
            .map(|a| {
                ListItem::from(Line::from(
                    a.arr
                        .iter()
                        .map(|a| {
                            let color = match a.0 {
                                TaskStringTag::Other => model.config.text_color,
                                TaskStringTag::Context => model.config.context_color,
                                TaskStringTag::Project => model.config.project_color,
                            };
                            Span::styled(a.1.as_str(), Style::new().set_style(color))
                        })
                        .collect::<Vec<Span>>(),
                ))
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
            .highlight_style(model.config.selected_text);
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
