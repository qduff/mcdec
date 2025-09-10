use crate::{
    app::{App, FocusedPane},
    renderers::render_function,
};
use itertools::Itertools;
use mcd::ILType;
use mcd_traits::display_with_resolver;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let left_vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Max(6)])
        .split(horizontal_layout[0]);

    // Functions
    let function_items: Vec<ListItem> = app
        .mcd
        .functions
        .iter()
        .map(|func| {
            let line = Line::from(vec![
                Span::styled(
                    format!("0x{:x} ", func.get_start_address().unwrap()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    func.get_name()
                        .iter()
                        .map(|v| display_with_resolver(v, &app.mcd.symbols))
                        .join("/"),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(
                    format!("({})", func.get_arg_count()),
                    Style::default().fg(Color::Yellow),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let focus_style = if matches!(app.focused_pane, FocusedPane::FunctionList) {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let functions_list = List::new(function_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Functions")
                .border_style(focus_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(
        functions_list,
        left_vertical_layout[0],
        &mut app.function_list_state.clone(),
    );

    // ILView
    let selected_function_index = app.function_list_state.selected().unwrap();
    let function = app.mcd.functions.get_mut(selected_function_index).unwrap();

    let focus_style = if matches!(app.focused_pane, FocusedPane::ILView) {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selected_il = ILType::all()
        .nth(app.il_list_state.selected().unwrap())
        .unwrap();

    let func_name = function
        .get_name()
        .iter()
        .map(|v| display_with_resolver(v, &app.mcd.symbols))
        .join("/");

    let mut decomp_text = String::new();
    match selected_il {
        ILType::Disassembly => {
            let _ = function.with_disassembly(|f| {
                use std::fmt::Write;
                write!(decomp_text, "{}", render_function(f, &app.mcd.symbols)).unwrap();
            });
        }
        ILType::SSA => {
            let _ = function.with_ssa(|f| {
                use std::fmt::Write;
                write!(decomp_text, "{}", render_function(f, &app.mcd.symbols)).unwrap();
            });
        }
    }

    let func_name_span = Span::styled(
        format!("function {func_name:}({})", function.get_arg_count()),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let mut combined_text = Text::from(Line::from(vec![func_name_span]));
    combined_text.extend(Text::raw(decomp_text));

    let decomp_view = Paragraph::new(combined_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(focus_style)
                .style(Style::default().fg(Color::White))
                .title(format!("{}", selected_il.to_string())),
        )
        .scroll((app.il_view_scroll, 0));
    f.render_widget(decomp_view, horizontal_layout[1]);

    // ILs
    let il_items: Vec<ListItem> = ILType::all()
        .map(|il_type| {
            let status = function.get_il_status(il_type);
            let line = Line::from(vec![
                Span::raw(format!("{:<13}", il_type.to_string())),
                Span::raw(status),
            ]);
            ListItem::new(line)
        })
        .collect();

    let il_focus_style = if matches!(app.focused_pane, FocusedPane::ILList) {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let il_list = List::new(il_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("ILs")
                .border_style(il_focus_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(
        il_list,
        left_vertical_layout[1],
        &mut app.il_list_state.clone(),
    );
}
