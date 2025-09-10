use crate::app::{App, FocusedPane};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search bar
    let search_text = if let Some(textarea) = &mut app.search_textarea {
        f.render_widget(textarea.widget(), layout[0]);
        textarea.lines()[0].clone()
    } else {
        let search_paragraph = Paragraph::new(app.symbol_search_text.as_str())
            .block(Block::default().borders(Borders::ALL).title("Filter (/)"));
        f.render_widget(search_paragraph, layout[0]);
        app.symbol_search_text.clone()
    };

    // Symbols table
    let focus_style = if matches!(app.focused_pane, FocusedPane::SymbolTable) {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let headers = Row::new(
        ["Source", "Address", "Name"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow))),
    )
    .height(1);

    let search_text_lower = search_text.to_lowercase();
    let rows = app.mcd.symbols.iter_symbols()
        .filter(|symbol| symbol.name.to_lowercase().contains(&search_text_lower))
        .map(|symbol| {
        Row::new(vec![
            Cell::from(format!("{:?}", symbol.source)),
            Cell::from(format!("{:}", symbol.key)),
            Cell::from(symbol.name.as_str()),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Percentage(100),
        ],
    )
    .header(headers)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Symbols")
            .border_style(focus_style),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, layout[1], &mut app.symbols_table_state);
}
