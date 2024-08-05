use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{palette::tailwind, Color, Style, Styled},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::{Model, RunningState};

const SELECTED_STYLE_FG: Color = tailwind::BLUE.c300;
const TEXT_COLOR: Color = tailwind::SLATE.c200;
const COMPLETED_TEXT_COLOR: Color = tailwind::GREEN.c500;

pub fn view(model: &mut Model, f: &mut Frame<'_>) {
    let outer_block = Block::new()
        .title("Todo")
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 1, 0));

    let outer_area = outer_block.inner(f.size());
    let chunks = Layout::default()
        .constraints([Constraint::Min(10), Constraint::Max(1)])
        .split(outer_area);
    f.render_widget(outer_block, f.size());
    match model.running_state {
        RunningState::Running => {
            let list_block = Block::new().borders(Borders::BOTTOM);
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
            .block(list_block)
            .highlight_style(SELECTED_STYLE_FG);

            f.render_stateful_widget(list, chunks[0], &mut model.state);
            f.render_widget(Line::raw("d: Toggle Todo; e: Edit, q: Quit"), chunks[1])
        }
        RunningState::Edit => {
            let layout = centered_rect(50, 50, chunks[0]);
            let width = layout.width.max(3) - 3;
            let input = model.input.as_mut().unwrap();
            let scroll = input.visual_scroll(width as usize);
            let input_widget = Paragraph::new(input.value())
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_widget, layout);
            f.set_cursor(
                //     // Put cursor past the end of the input text
                layout.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
                //     // Move one line down, from the border to the input line
                layout.y + 1,
            );
            f.render_widget(
                Line::raw("C-d: Save and Exit Edit Mode; Esc - Exit Edit Mode"),
                chunks[1],
            );
        }
        RunningState::Done => unreachable!(),
    };
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
