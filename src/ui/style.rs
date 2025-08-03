// SPDX-License-Identifier: MIT

use crate::task::Priority;
use ratatui::style::{Color, Modifier, Style};

pub const COLOR_PALETTE: [Color; 16] = [
    Color::from_u32(0x2e3440), // #2e3440 0
    Color::from_u32(0x3b4252), // #3b4252 1
    Color::from_u32(0x434c5e), // #434c5e 2
    Color::from_u32(0x4c566a), // #4c566a 3
    Color::from_u32(0xd8dee9), // #d8dee9 4
    Color::from_u32(0xe5e9f0), // #e5e9f0 5
    Color::from_u32(0xeceff4), // #eceff4 6
    Color::from_u32(0x8fbcbb), // #8fbcbb 7
    Color::from_u32(0x88c0d0), // #88c0d0 8
    Color::from_u32(0x81a1c1), // #81a1c1 9
    Color::from_u32(0x5e81ac), // #5e81ac 10
    Color::from_u32(0xbf616a), // #bf616a 11
    Color::from_u32(0xd08770), // #d08770 12
    Color::from_u32(0xebcb8b), // #ebcb8b 13
    Color::from_u32(0xa3be8c), // #a3be8c 14
    Color::from_u32(0xb48ead), // #b48ead 15
];

pub const URL_UNDER_MOUSE_COLOR: Color = COLOR_PALETTE[10];
pub const URL_COLOR: Color = COLOR_PALETTE[4];

pub const OVERDUE_TASK_FG: Color = COLOR_PALETTE[11];
pub const TODAY_TASK_FG: Color = COLOR_PALETTE[4];
pub const FUTURE_TASK_FG: Color = COLOR_PALETTE[14];
pub const NO_DATE_TASK_FG: Color = TODAY_TASK_FG;
pub const DESCRIPTION_KEY_COLOR: Color = COLOR_PALETTE[7];
pub const DESCRIPTION_VALUE_COLOR: Color = COLOR_PALETTE[4];
pub const PROVIDER_COLORS: &[Color] = &[
    COLOR_PALETTE[7],
    COLOR_PALETTE[8],
    COLOR_PALETTE[9],
    COLOR_PALETTE[10],
    COLOR_PALETTE[11],
    COLOR_PALETTE[12],
    COLOR_PALETTE[13],
    COLOR_PALETTE[14],
    COLOR_PALETTE[15],
];

pub const FOOTER_KEYS_HELP_COLOR: Color = COLOR_PALETTE[4];
pub const FOOTER_DATETIME_LABEL_FG: Color = COLOR_PALETTE[13];
pub const FOOTER_DATETIME_FG: Color = COLOR_PALETTE[7];
pub const FOOTER_KEYS_LABEL_FG: Color = COLOR_PALETTE[14];
pub const FOOTER_KEYS_FG: Color = COLOR_PALETTE[11];

pub const HEADER_KEY_SELECTED_FG: Color = COLOR_PALETTE[11];
pub const HEADER_KEY_FG: Color = COLOR_PALETTE[1];

pub const ACTIVE_BLOCK_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[0]).bg(COLOR_PALETTE[14]);
pub const INACTIVE_BLOCK_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[4]).bg(COLOR_PALETTE[10]);
pub const SELECTED_ROW_STYLE: Style = DEFAULT_STYLE.bg(COLOR_PALETTE[2]).add_modifier(Modifier::BOLD);
pub const REGULAR_ROW_STYLE: Style = DEFAULT_STYLE;
pub const REGULAR_TEXT_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[4]);
pub const INLINE_CODE_TEXT_STYLE: Style = DEFAULT_STYLE.add_modifier(Modifier::ITALIC);
pub const LABEL_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[7]).add_modifier(Modifier::ITALIC);

pub const DATE_TIME_EDITOR_ACTIVE_ELEMENT: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[0]).bg(COLOR_PALETTE[4]);
pub const DATE_TIME_EDITOR_INACTIVE_ELEMENT: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[4]).bg(COLOR_PALETTE[3]);

pub const ACTIVE_BUTTON_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[0]).bg(COLOR_PALETTE[7]);
pub const INACTIVE_BUTTON_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[4]);

pub const WARNING_TEXT_STYLE: Style = DEFAULT_STYLE.fg(COLOR_PALETTE[13]);

pub const BORDER_COLOR: Color = COLOR_PALETTE[6];

// pub const DEFAULT_STYLE: Style = Style::new().bg(Color::White).fg(Color::Black);
pub static DEFAULT_STYLE: Style = Style::new().bg(COLOR_PALETTE[0]).fg(COLOR_PALETTE[4]);

pub fn priority_color(p: &Priority) -> Color {
    match p {
        Priority::Lowest => COLOR_PALETTE[1],
        Priority::Low => COLOR_PALETTE[3],
        Priority::Normal => COLOR_PALETTE[14],
        Priority::Medium => COLOR_PALETTE[13],
        Priority::High => COLOR_PALETTE[12],
        Priority::Highest => COLOR_PALETTE[11],
    }
}
