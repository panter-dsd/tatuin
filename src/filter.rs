// SPDX-License-Identifier: MIT

use clap::ValueEnum;

#[derive(Clone, PartialEq, Eq, ValueEnum, Debug)]
pub enum FilterState {
    Completed,
    Uncompleted,
    InProgress,
    Unknown,
}

impl std::fmt::Display for FilterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum, Ord, PartialOrd)]
pub enum Due {
    Overdue,
    Today,
    Future,
    NoDate,
}

impl std::fmt::Display for Due {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Filter {
    pub states: Vec<FilterState>,
    pub due: Vec<Due>,
}
