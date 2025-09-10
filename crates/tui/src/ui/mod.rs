// src/ui/mod.rs

mod code_listing_tab;
mod popups;
mod symbols_tab;

use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs},
};

pub fn ui(f: &mut Frame, app: &mut App) {
    // Main layout
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.size());

    // --- Tabs ---
    let titles: Vec<_> = app.tabs.iter().cloned().map(Line::from).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.tab_index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    f.render_widget(tabs, main_layout[0]);

    // --- Content based on tab ---
    match app.tab_index {
        0 => code_listing_tab::render(f, app, main_layout[1]),
        1 => symbols_tab::render(f, app, main_layout[1]),
        _ => {
            panic!()
        }
    }

    // --- Popups ---
    if let Some(textarea) = &app.renaming_textarea {
        popups::render_rename_popup(f, textarea);
    }
    if let Some(textarea) = &app.search_textarea {
        if app.tab_index != 1 {
            popups::render_rename_popup(f, textarea);
        }
    }
    if let Some(insert_state) = &app.insert_state {
        popups::render_insert_popup(f, insert_state);
    }
}
