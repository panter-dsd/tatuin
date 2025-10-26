use std::fmt::Display;

use tatuin_core::task::TaskNameProvider as TaskNameProviderTrait;

use crate::obsidian::md_file::TAG_RE;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct TaskNameProvider {
    name: String,
    display: String,
}

impl TaskNameProvider {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            display: Self::clear_tags(name),
        }
    }

    fn clear_tags(name: &str) -> String {
        TAG_RE.replace_all(name, "").to_string()
    }
}

impl<T> From<T> for TaskNameProvider
where
    T: Display,
{
    fn from(value: T) -> Self {
        Self::new(value.to_string().as_str())
    }
}

impl TaskNameProviderTrait for TaskNameProvider {
    fn raw(&self) -> String {
        self.name.clone()
    }

    fn display(&self) -> String {
        self.display.clone()
    }
}
