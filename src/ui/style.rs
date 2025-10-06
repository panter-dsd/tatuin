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
    EnabledButtonFG,
    EnabledButtonBG,
    DisabledButtonFG,
    DisabledButtonBG,
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
    if let Some(m) = &*THEME_MAP.read().unwrap()
        && let Some(c) = m.get(&element)
    {
        return *c;
    }

    const DEFAULT_FG: Color = Color::White;
    const DEFAULT_BG: Color = Color::Black;

    use ColorElement::*;
    match element {
        DefaultBG => DEFAULT_BG,
        DefaultFG => DEFAULT_FG,
        TaskRowDueFG => Color::Blue,
        TaskRowPlaceFG => Color::Yellow,
        UrlFG => DEFAULT_FG,
        UrlUnderMouseFG => Color::Blue,
        ActiveBlockFG => SLATE.c100,
        ActiveBlockBG => GREEN.c800,
        InactiveBlockFG => SLATE.c100,
        InactiveBlockBG => BLUE.c800,
        OverdueTaskFG => Color::LightRed,
        TodayTaskFG => DEFAULT_FG,
        FutureTaskFG => Color::LightGreen,
        NoDateTaskFG => DEFAULT_FG,
        DescriptionKeyFG => Color::Blue,
        DescriptionValueFG => DEFAULT_FG,
        Provider1FG => Color::Green,
        Provider2FG => Color::Magenta,
        Provider3FG => Color::Cyan,
        Provider4FG => Color::Yellow,
        Provider5FG => Color::Blue,
        Provider6FG => Color::Red,
        FooterKeysHelpFG => DEFAULT_FG,
        FooterDatetimeLabelFG => Color::Yellow,
        FooterDatetimeFG => Color::LightCyan,
        FooterKeysLabelFG => Color::Green,
        FooterKeysFG => Color::LightRed,
        HeaderKeySelectedFG => Color::LightRed,
        HeaderKeyFG => Color::Rgb(255, 192, 203),
        SelectedRowBG => SLATE.c800,
        RegularTextFG => DEFAULT_FG,
        LabelFG => Color::Cyan,
        DateTimeEditorActiveElementFG => DEFAULT_BG,
        DateTimeEditorActiveElementBG => Color::LightBlue,
        DateTimeEditorInactiveElementFG => Color::Black,
        DateTimeEditorInactiveElementBG => Color::Gray,
        ActiveButtonFG => SLATE.c100,
        ActiveButtonBG => GREEN.c800,
        EnabledButtonFG => DEFAULT_FG,
        EnabledButtonBG => DEFAULT_BG,
        DisabledButtonFG => DEFAULT_FG,
        DisabledButtonBG => Color::DarkGray,
        InactiveButtonFG => DEFAULT_FG,
        WarningTextFG => Color::Yellow,
        BorderColor => DEFAULT_FG,
        LowestPriorityFG => Color::DarkGray,
        LowPriorityFG => Color::Gray,
        NormalPriorityFG => Color::LightGreen,
        MediumPriorityFG => Color::Rgb(255, 192, 203),
        HighPriorityFG => Color::LightRed,
        HighestPriorityFG => Color::Red,
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
            if let Ok(k) = k
                && let Ok(v) = v
            {
                theme_map.insert(k, v);
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

pub fn disabled_button_style() -> Style {
    default_style()
        .bg(element_color(ColorElement::DisabledButtonBG))
        .fg(element_color(ColorElement::DisabledButtonFG))
}

pub fn enabled_button_style() -> Style {
    default_style()
        .bg(element_color(ColorElement::EnabledButtonBG))
        .fg(element_color(ColorElement::EnabledButtonFG))
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
