// SPDX-License-Identifier: MIT

use std::{collections::HashMap, str::FromStr, sync::RwLock};

use crate::task::Priority;
use clap::ValueEnum;
use ratatui::style::{
    Color, Modifier, Style,
    palette::tailwind::{BLUE, GREEN, SLATE},
};

#[derive(PartialEq, Eq, std::hash::Hash, ValueEnum, Copy, Clone, Debug)]
#[clap(rename_all = "snake_case")]
enum ColorElement {
    DefaultBG,
    DefaultFG,
    TaskRowDueFG,
    TaskRowPlaceFG,
    UrlFG,
    UrlUnderMouseFG,
    ActiveBlockFG,
    ActiveBlockBG,
    InactiveBlockFG,
    InactiveBlockBG,
    OverdueTaskFG,
    TodayTaskFG,
    FutureTaskFG,
    NoDateTaskFG,
    DescriptionKeyFG,
    DescriptionValueFG,
    Provider1FG,
    Provider2FG,
    Provider3FG,
    Provider4FG,
    Provider5FG,
    Provider6FG,
    FooterKeysHelpFG,
    FooterDatetimeLabelFG,
    FooterDatetimeFG,
    FooterKeysLabelFG,
    FooterKeysFG,
    HeaderKeySelectedFG,
    HeaderKeyFG,
    SelectedRowBG,
    RegularTextFG,
    LabelFG,
    DateTimeEditorActiveElementFG,
    DateTimeEditorActiveElementBG,
    DateTimeEditorInactiveElementFG,
    DateTimeEditorInactiveElementBG,
    ActiveButtonFG,
    ActiveButtonBG,
    InactiveButtonFG,
    WarningTextFG,
    BorderColor,
    LowestPriorityFG,
    LowPriorityFG,
    NormalPriorityFG,
    MediumPriorityFG,
    HighPriorityFG,
    HighestPriorityFG,
}

static THEME_MAP: RwLock<Option<HashMap<ColorElement, Color>>> = RwLock::new(None);

fn element_color(element: ColorElement) -> Color {
    if let Some(m) = &*THEME_MAP.read().unwrap() {
        if let Some(c) = m.get(&element) {
            return *c;
        }
    }

    match element {
        ColorElement::DefaultBG => Color::Black,
        ColorElement::DefaultFG => Color::White,
        ColorElement::TaskRowDueFG => Color::Blue,
        ColorElement::TaskRowPlaceFG => Color::Yellow,
        ColorElement::UrlFG => Color::White,
        ColorElement::UrlUnderMouseFG => Color::Blue,
        ColorElement::ActiveBlockFG => SLATE.c100,
        ColorElement::ActiveBlockBG => GREEN.c800,
        ColorElement::InactiveBlockFG => SLATE.c100,
        ColorElement::InactiveBlockBG => BLUE.c800,
        ColorElement::OverdueTaskFG => Color::LightRed,
        ColorElement::TodayTaskFG => Color::White,
        ColorElement::FutureTaskFG => Color::LightGreen,
        ColorElement::NoDateTaskFG => Color::White,
        ColorElement::DescriptionKeyFG => Color::Blue,
        ColorElement::DescriptionValueFG => Color::White,
        ColorElement::Provider1FG => Color::Green,
        ColorElement::Provider2FG => Color::Magenta,
        ColorElement::Provider3FG => Color::Cyan,
        ColorElement::Provider4FG => Color::Yellow,
        ColorElement::Provider5FG => Color::Blue,
        ColorElement::Provider6FG => Color::Red,
        ColorElement::FooterKeysHelpFG => Color::White,
        ColorElement::FooterDatetimeLabelFG => Color::Yellow,
        ColorElement::FooterDatetimeFG => Color::LightCyan,
        ColorElement::FooterKeysLabelFG => Color::Green,
        ColorElement::FooterKeysFG => Color::LightRed,
        ColorElement::HeaderKeySelectedFG => Color::LightRed,
        ColorElement::HeaderKeyFG => Color::Rgb(255, 192, 203),
        ColorElement::SelectedRowBG => SLATE.c800,
        ColorElement::RegularTextFG => Color::White,
        ColorElement::LabelFG => Color::Cyan,
        ColorElement::DateTimeEditorActiveElementFG => Color::Black,
        ColorElement::DateTimeEditorActiveElementBG => Color::LightBlue,
        ColorElement::DateTimeEditorInactiveElementFG => Color::Black,
        ColorElement::DateTimeEditorInactiveElementBG => Color::Gray,
        ColorElement::ActiveButtonFG => SLATE.c100,
        ColorElement::ActiveButtonBG => GREEN.c800,
        ColorElement::InactiveButtonFG => Color::White,
        ColorElement::WarningTextFG => Color::Yellow,
        ColorElement::BorderColor => Color::White,
        ColorElement::LowestPriorityFG => Color::DarkGray,
        ColorElement::LowPriorityFG => Color::Gray,
        ColorElement::NormalPriorityFG => Color::LightGreen,
        ColorElement::MediumPriorityFG => Color::Rgb(255, 192, 203),
        ColorElement::HighPriorityFG => Color::LightRed,
        ColorElement::HighestPriorityFG => Color::Red,
    }
}

pub fn load_theme(file_path: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(file_path)?;

    let mut theme_map = HashMap::new();

    for line in data.lines() {
        if let Some(l) = line.split_once('=') {
            let k = ColorElement::from_str(l.0.trim(), true);
            let v = Color::from_str(l.1.trim());
            if v.is_err() || v.is_err() {
                println!("Can't parse line `{line}`: {k:?} {v:?}");
            }
            if let Ok(k) = k {
                if let Ok(v) = v {
                    theme_map.insert(k, v);
                }
            }
        }
    }

    *THEME_MAP.write().unwrap() = Some(theme_map);

    Ok(())
}

pub fn due_color() -> Color {
    element_color(ColorElement::TaskRowDueFG)
}

pub fn place_color() -> Color {
    element_color(ColorElement::TaskRowPlaceFG)
}

pub fn url_under_mouse_color() -> Color {
    element_color(ColorElement::UrlUnderMouseFG)
}

pub fn url_color() -> Color {
    element_color(ColorElement::UrlFG)
}

pub fn overdue_task_fg() -> Color {
    element_color(ColorElement::OverdueTaskFG)
}
pub fn today_task_fg() -> Color {
    element_color(ColorElement::TodayTaskFG)
}
pub fn future_task_fg() -> Color {
    element_color(ColorElement::FutureTaskFG)
}
pub fn no_date_task_fg() -> Color {
    element_color(ColorElement::NoDateTaskFG)
}
pub fn description_key_color() -> Color {
    element_color(ColorElement::DescriptionKeyFG)
}
pub fn description_value_color() -> Color {
    element_color(ColorElement::DescriptionValueFG)
}

pub fn provider_colors() -> Vec<Color> {
    vec![
        element_color(ColorElement::Provider1FG),
        element_color(ColorElement::Provider2FG),
        element_color(ColorElement::Provider3FG),
        element_color(ColorElement::Provider4FG),
        element_color(ColorElement::Provider5FG),
        element_color(ColorElement::Provider6FG),
    ]
}

pub fn footer_keys_help_color() -> Color {
    element_color(ColorElement::FooterKeysHelpFG)
}
pub fn footer_datetime_label_fg() -> Color {
    element_color(ColorElement::FooterDatetimeLabelFG)
}
pub fn footer_datetime_fg() -> Color {
    element_color(ColorElement::FooterDatetimeFG)
}
pub fn footer_keys_label_fg() -> Color {
    element_color(ColorElement::FooterKeysLabelFG)
}
pub fn footer_keys_fg() -> Color {
    element_color(ColorElement::FooterKeysFG)
}

pub fn header_key_selected_fg() -> Color {
    element_color(ColorElement::HeaderKeySelectedFG)
}
pub fn header_key_fg() -> Color {
    element_color(ColorElement::HeaderKeyFG)
}

pub fn active_block_style() -> Style {
    default_style()
        .fg(element_color(ColorElement::ActiveBlockFG))
        .bg(element_color(ColorElement::ActiveBlockBG))
}
pub fn inactive_block_style() -> Style {
    default_style()
        .fg(element_color(ColorElement::InactiveBlockFG))
        .bg(element_color(ColorElement::InactiveBlockBG))
}

pub fn selected_row_style() -> Style {
    default_style()
        .bg(element_color(ColorElement::SelectedRowBG))
        .add_modifier(Modifier::BOLD)
}
pub fn regular_row_style() -> Style {
    default_style()
}
pub fn regular_text_style() -> Style {
    default_style().fg(element_color(ColorElement::RegularTextFG))
}
pub fn inline_code_text_style() -> Style {
    default_style().add_modifier(Modifier::ITALIC)
}
pub fn label_style() -> Style {
    default_style()
        .fg(element_color(ColorElement::LabelFG))
        .add_modifier(Modifier::ITALIC)
}

pub fn date_time_editor_active_element() -> Style {
    default_style()
        .fg(element_color(ColorElement::DateTimeEditorActiveElementFG))
        .bg(element_color(ColorElement::DateTimeEditorActiveElementBG))
}
pub fn date_time_editor_inactive_element() -> Style {
    default_style()
        .fg(element_color(ColorElement::DateTimeEditorInactiveElementFG))
        .bg(element_color(ColorElement::DateTimeEditorInactiveElementBG))
}

pub fn active_button_style() -> Style {
    default_style()
        .fg(element_color(ColorElement::ActiveButtonFG))
        .bg(element_color(ColorElement::ActiveButtonBG))
}
pub fn inactive_button_style() -> Style {
    default_style().fg(element_color(ColorElement::InactiveButtonFG))
}

pub fn warning_text_style() -> Style {
    default_style().fg(element_color(ColorElement::WarningTextFG))
}

pub fn border_color() -> Color {
    element_color(ColorElement::BorderColor)
}

pub fn default_style() -> Style {
    Style::new()
        .bg(element_color(ColorElement::DefaultBG))
        .fg(element_color(ColorElement::DefaultFG))
}

pub fn priority_color(p: &Priority) -> Color {
    match p {
        Priority::Lowest => element_color(ColorElement::LowestPriorityFG),
        Priority::Low => element_color(ColorElement::LowPriorityFG),
        Priority::Normal => element_color(ColorElement::NormalPriorityFG),
        Priority::Medium => element_color(ColorElement::MediumPriorityFG),
        Priority::High => element_color(ColorElement::HighPriorityFG),
        Priority::Highest => element_color(ColorElement::HighestPriorityFG),
    }
}
