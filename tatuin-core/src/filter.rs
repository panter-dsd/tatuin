// SPDX-License-Identifier: MIT

use super::task::{State, Task as TaskTrait, due_group};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, ValueEnum, Debug, Serialize, Deserialize)]
pub enum FilterState {
    Completed,
    Uncompleted,
    InProgress,
    Unknown,
}

impl FilterState {
    pub fn values() -> Vec<Self> {
        vec![
            FilterState::Completed,
            FilterState::Uncompleted,
            FilterState::InProgress,
            FilterState::Unknown,
        ]
    }
}

impl std::fmt::Display for FilterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<State> for FilterState {
    fn from(s: State) -> Self {
        match s {
            State::Completed => FilterState::Completed,
            State::Uncompleted => FilterState::Uncompleted,
            State::InProgress => FilterState::InProgress,
            State::Unknown(_) => FilterState::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Due {
    Overdue,
    Today,
    Future,
    NoDate,
}

impl Due {
    pub fn values() -> Vec<Self> {
        vec![Due::Overdue, Due::Today, Due::Future, Due::NoDate]
    }
}

impl std::fmt::Display for Due {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize, Default, Debug)]
pub struct Filter {
    pub states: Vec<FilterState>,
    pub due: Vec<Due>,
}

impl Filter {
    pub fn accept(&self, t: &dyn TaskTrait) -> bool {
        if !self.states.contains(&t.state().into()) {
            return false;
        }

        if !self.due.contains(&due_group(&t.due())) && !self.due.contains(&due_group(&t.scheduled())) {
            return false;
        }

        true
    }

    pub fn full_filter() -> Self {
        Self {
            states: FilterState::values(),
            due: Due::values(),
        }
    }
}
