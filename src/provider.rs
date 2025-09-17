use ratatui::style::Color;
use tatuin_core::{
    provider::{Capabilities, ProviderTrait},
    task::Priority,
    types::ArcRwLock,
};

#[derive(Clone)]
pub struct Provider {
    pub name: String,
    pub type_name: String,
    pub color: Color,
    pub capabilities: Capabilities,
    pub supported_priorities: Vec<Priority>,
    pub provider: ArcRwLock<Box<dyn ProviderTrait>>,
}
