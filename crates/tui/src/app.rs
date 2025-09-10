// src/app.rs

use mcd::{ILType, MCD};

use ratatui::{
    style::Style,
    widgets::{Block, ListState, TableState},
};
use tui_textarea::TextArea;

pub enum FocusedPane {
    FunctionList,
    ILList,
    ILView,
    SymbolTable,
}

pub enum InsertFocus {
    Address,
    Name,
}

pub struct InsertState<'a> {
    pub address: TextArea<'a>,
    pub name: TextArea<'a>,
    pub focus: InsertFocus,
}

impl<'a> InsertState<'a> {
    pub fn new() -> Self {
        let mut address_input = TextArea::default();
        address_input.set_placeholder_text("Enter address (e.g., 0x1234)");
        address_input.set_block(Block::default().borders(ratatui::widgets::Borders::ALL).title("Address"));

        let mut name_input = TextArea::default();
        name_input.set_placeholder_text("Enter name");
        name_input.set_block(Block::default().borders(ratatui::widgets::Borders::ALL).title("Name"));

        Self {
            address: address_input,
            name: name_input,
            focus: InsertFocus::Address,
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            InsertFocus::Address => InsertFocus::Name,
            InsertFocus::Name => InsertFocus::Address,
        }
    }
}


pub struct App<'a> {
    pub mcd: MCD,

    pub should_quit: bool,
    pub tabs: Vec<&'a str>,
    pub tab_index: usize,

    // code listing
    pub function_list_state: ListState,
    pub il_list_state: ListState,
    pub il_view_scroll: u16,
    pub focused_pane: FocusedPane,
    
    // Symbols
    pub symbols_table_state: TableState,
    pub symbol_search_text: String,
    pub renaming_symbol_key: Option<u32>,
    pub insert_state: Option<InsertState<'a>>,

    // Popups
    pub renaming_textarea: Option<TextArea<'a>>,
    pub search_textarea: Option<TextArea<'a>>,
}

impl<'a> App<'a> {
    pub fn new(mcd: MCD) -> App<'a> {
        let mut function_list_state = ListState::default();
        function_list_state.select(Some(0));

        let mut il_list_state = ListState::default();
        il_list_state.select(Some(0));

        let mut symbols_table_state = TableState::default();
        symbols_table_state.select(Some(0));

        App {
            mcd,
            should_quit: false,
            tabs: vec!["Code Listing", "Symbols"],
            tab_index: 0,
            function_list_state,
            il_list_state,
            il_view_scroll: 0,
            focused_pane: FocusedPane::FunctionList,
            symbols_table_state,
            symbol_search_text: String::new(),
            renaming_symbol_key: None,
            insert_state: None,
            renaming_textarea: None,
            search_textarea: None,
        }
    }

    pub fn on_quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % self.tabs.len();
        self.update_focused_pane_on_tab_change();
    }

    pub fn prev_tab(&mut self) {
        if self.tab_index > 0 {
            self.tab_index -= 1;
        } else {
            self.tab_index = self.tabs.len() - 1;
        }
        self.update_focused_pane_on_tab_change();
    }

    fn update_focused_pane_on_tab_change(&mut self) {
        self.focused_pane = match self.tab_index {
            0 => FocusedPane::FunctionList,
            1 => FocusedPane::SymbolTable,
            _ => FocusedPane::FunctionList,
        }
    }

    pub fn cycle_focused_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::FunctionList => FocusedPane::ILList,
            FocusedPane::ILList => FocusedPane::ILView,
            FocusedPane::ILView => FocusedPane::FunctionList,
            FocusedPane::SymbolTable => FocusedPane::SymbolTable,
        };
    }

    pub fn change_pos(&mut self, offset: i32) {
        let adjust_selection = |state: &mut ListState, list_len: usize| {
            if list_len == 0 {
                return;
            }
            let current = state.selected().unwrap_or(0);
            let mut new_pos = current as i32 + offset;
            new_pos = new_pos.rem_euclid(list_len as i32);
            state.select(Some(new_pos as usize));
        };

        match self.focused_pane {
            FocusedPane::FunctionList => {
                let len = self.mcd.functions.len();
                adjust_selection(&mut self.function_list_state, len);
                self.il_view_scroll = 0;
            }
            FocusedPane::ILList => {
                self.il_view_scroll = 0;
                let len = ILType::all().count();
                adjust_selection(&mut self.il_list_state, len);
            }
            FocusedPane::ILView => {
                if offset > 0 {
                    self.il_view_scroll = self.il_view_scroll.saturating_add(offset as u16);
                } else {
                    self.il_view_scroll = self.il_view_scroll.saturating_sub(-offset as u16);
                }
            }
            _ => {}
        }
    }

    // pub fn start_renaming(&mut self) {
    //     if let Some(selected_index) = self.function_list_state.selected() {
    //         let current_name = self.mcd.functions[selected_index].get_name();
    //         let mut textarea = TextArea::new(vec![current_name.clone()]);
    //         textarea.set_placeholder_text(current_name);
    //         textarea.set_cursor_line_style(Style::default());
    //         textarea.set_block(
    //             Block::default()
    //                 .borders(ratatui::widgets::Borders::ALL)
    //                 .border_style(Style::new().fg(ratatui::style::Color::White))
    //                 .title("Choose name"),
    //         );
    //         textarea.select_all();
    //         self.renaming_textarea = Some(textarea);
    //     }
    // }
    // pub fn confirm_rename(&mut self) {
    //     if let Some(textarea) = self.renaming_textarea.take() {
    //         // .take() removes it from the Option
    //         if let Some(selected_index) = self.function_list_state.selected() {
    //             // Since it's a single line, we take the first line
    //             let new_name = textarea.lines()[0].clone();
    //             if !new_name.is_empty() {
    //                 if let Some(val) = self.mcd.functions.get_mut(selected_index) {
    //                     todo!();
    //                     // val.set_name(new_name);
    //                 };
    //             }
    //         }
    //     }
    // }

    pub fn cancel_rename(&mut self) {
        self.renaming_textarea = None;
    }
}
