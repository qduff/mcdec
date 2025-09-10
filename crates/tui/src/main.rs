mod app;
mod model;
mod ui;
mod renderers;

use app::{App, InsertFocus, InsertState};
use clap::Parser as ClapParser;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mcd::MCD;
use prgparser::{BinaryReader, Parser};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::ListState,
    Terminal,
};
use std::{
    error::Error,
    fs::File,
    io::{self, BufReader},
    time::Duration,
};
use tui_textarea::TextArea;

#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the prg file
    path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let f = File::open(args.path).unwrap();
    let len = f.metadata().unwrap().len();
    let mut buf_reader = BufReader::new(f);
    let binary_reader = BinaryReader::new(&mut buf_reader, len);

    let parsed = Parser::new(binary_reader).parse().unwrap();
    let mcd = MCD::new(parsed);

    // --- Terminal setup ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    let mut app = App::new(mcd);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

// TODO make more efficient
fn handle_list_selection(app: &mut App, move_down: bool) {
    let all_symbols: Vec<_> = app.mcd.symbols.iter_symbols().collect();
    let filtered_indices: Vec<usize> = all_symbols
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.name
                .to_lowercase()
                .contains(&app.symbol_search_text.to_lowercase())
        })
        .map(|(i, _)| i)
        .collect();
    let len = filtered_indices.len();
    if len == 0 {
        app.symbols_table_state.select(None);
    } else {
        let current = app.symbols_table_state.selected().unwrap_or(0);
        let new_pos = if move_down {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        app.symbols_table_state.select(Some(new_pos));
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            let event = event::read()?;

            if let Some(insert_state) = app.insert_state.as_mut() {
                match event {
                    Event::Key(key) if key.code == KeyCode::Esc => {
                        app.insert_state = None;
                    }
                    Event::Key(key) if key.code == KeyCode::Enter => {
                        let addr_str = insert_state.address.lines()[0].trim();
                        let name = insert_state.name.lines()[0].clone();
                        if !name.is_empty() && !addr_str.is_empty() {
                            let addr = if let Some(hex) = addr_str.strip_prefix("0x") {
                                u32::from_str_radix(hex, 16).ok()
                            } else {
                                addr_str.parse::<u32>().ok()
                            };
                            if let Some(addr) = addr {
                                app.mcd.symbols.set_symbol_name(addr, name);
                                app.insert_state = None;
                            }
                        }
                    }
                    Event::Key(key) if key.code == KeyCode::Tab => {
                        insert_state.toggle_focus();
                    }
                    Event::Key(key) => match insert_state.focus {
                        InsertFocus::Address => {
                            insert_state.address.input(key);
                        }
                        InsertFocus::Name => {
                            insert_state.name.input(key);
                        }
                    },
                    _ => {}
                }
            } else if let Some(textarea) = app.renaming_textarea.as_mut() {
                match event {
                    Event::Key(key) if key.code == KeyCode::Esc => {
                        app.cancel_rename();
                        app.renaming_symbol_key = None;
                    }
                    Event::Key(key) if key.code == KeyCode::Enter => {
                        let new_name = textarea.lines()[0].clone();
                        if !new_name.is_empty() {
                            if let Some(key) = app.renaming_symbol_key.take() {
                                app.mcd.symbols.set_symbol_name(key, new_name);
                            }
                            app.cancel_rename();
                        }
                    }
                    Event::Key(key) => {
                        textarea.input(key);
                    }
                    _ => {}
                }
            } else if let Some(textarea) = app.search_textarea.as_mut() {
                match event {
                    Event::Key(key) if key.code == KeyCode::Esc => {
                        app.search_textarea = None;
                    }
                    Event::Key(key) if key.code == KeyCode::Enter => {
                        app.symbol_search_text = textarea.lines()[0].clone();
                        app.search_textarea = None;
                    }
                    Event::Key(key) => {
                        textarea.input(key);
                    }
                    _ => {}
                }
            } else {
                // --- Normal mode event handling ---
                if let Event::Key(key) = event { match key.code {
                    KeyCode::Char('q') => app.on_quit(),
                    KeyCode::PageDown => app.next_tab(),
                    KeyCode::PageUp => app.prev_tab(),

                    // Tab-specific keys
                    _ => match app.tab_index {
                        0 => match key.code {
                            KeyCode::Down => app.change_pos(1),
                            KeyCode::Up => app.change_pos(-1),
                            KeyCode::Tab => app.cycle_focused_pane(),
                            _ => {}
                        },
                        1 => match key.code {
                            KeyCode::Down => handle_list_selection(app, true),
                            KeyCode::Up => handle_list_selection(app, false),
                            KeyCode::Char('n') => {
                                if let Some(selected_index) = app.symbols_table_state.selected() {
                                    let all_symbols: Vec<_> = app.mcd.symbols.iter_symbols().collect();
                                    let filtered_indices: Vec<usize> = all_symbols
                                        .iter()
                                        .enumerate()
                                        .filter(|(_, s)| {
                                            s.name.to_lowercase().contains(&app.symbol_search_text.to_lowercase())
                                        })
                                        .map(|(i, _)| i)
                                        .collect();

                                    if let Some(&original_index) = filtered_indices.get(selected_index) {
                                        let symbol_to_rename = &all_symbols[original_index];
                                        app.renaming_symbol_key = Some(symbol_to_rename.key);

                                        let mut textarea = TextArea::new(vec![symbol_to_rename.name.clone()]);
                                        textarea.set_block(
                                            ratatui::widgets::Block::default()
                                                .borders(ratatui::widgets::Borders::ALL)
                                                .title("Rename Symbol"),
                                        );
                                        textarea.select_all();
                                        app.renaming_textarea = Some(textarea);
                                    }
                                }
                            }
                            KeyCode::Char('i') => {
                                app.insert_state = Some(InsertState::new());
                            }
                            KeyCode::Char('/') => {
                                let mut textarea = TextArea::new(vec![app.symbol_search_text.clone()]);
                                textarea.set_block(
                                    ratatui::widgets::Block::default()
                                        .borders(ratatui::widgets::Borders::ALL)
                                        .title("Search"),
                                );
                                textarea.select_all();
                                app.search_textarea = Some(textarea);
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                } }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
