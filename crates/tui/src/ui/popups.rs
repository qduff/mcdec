use crate::app::{InsertFocus, InsertState};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear},
};
use tui_textarea::TextArea;

pub fn render_rename_popup(f: &mut Frame, textarea: &TextArea) {
    let area = centered_rect(60, 15, f.area());
    f.render_widget(Clear, area); 

    let textarea_widget = textarea.widget();
    f.render_widget(textarea_widget, area);
}

pub fn render_insert_popup(f: &mut Frame, state: &InsertState) {
    let area = centered_rect(60, 40, f.area());
    f.render_widget(Clear, area);

    let insert_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .margin(1)
        .split(area);

    let popup_block = Block::default()
        .title("Insert New Symbol (Enter to confirm, Esc to cancel)")
        .borders(Borders::ALL);
    f.render_widget(popup_block, area);

    let mut address_input = state.address.clone();
    if matches!(state.focus, InsertFocus::Address) {
        address_input.set_block(
            address_input
                .block()
                .cloned()
                .unwrap()
                .border_style(Style::default().fg(Color::Yellow)),
        );
    }
    f.render_widget(address_input.widget(), insert_layout[0]);

    let mut name_input = state.name.clone();
    if matches!(state.focus, InsertFocus::Name) {
        name_input.set_block(
            name_input
                .block()
                .cloned()
                .unwrap()
                .border_style(Style::default().fg(Color::Yellow)),
        );
    }
    f.render_widget(name_input.widget(), insert_layout[1]);
}

/// Helper function to create a centered rect of a certain size.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
