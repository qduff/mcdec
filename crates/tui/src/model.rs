// src/model.rs

use ratatui::style::Color;

#[derive(Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub address: u64,
    pub args: u8,
}

#[derive(Clone)]
pub enum ILStatus {
    Ok,
    Pending,
    Error,
}

impl ILStatus {
    pub fn to_string(&self) -> &'static str {
        match self {
            ILStatus::Ok => "OK",
            ILStatus::Pending => "PENDING",
            ILStatus::Error => "ERROR",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ILStatus::Ok => Color::Green,
            ILStatus::Pending => Color::Yellow,
            ILStatus::Error => Color::Red,
        }
    }
}

#[derive(Clone)]
pub struct ILInfo {
    pub name: String,
    pub status: ILStatus,
}

#[derive(Clone)]
pub struct Symbol {
    pub id: String,
    pub value: String,
}