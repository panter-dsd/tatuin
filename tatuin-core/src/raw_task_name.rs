// SPDX-License-Identifier: MIT

use std::fmt::Display;

use crate::task::TaskNameProvider;

#[derive(PartialEq, Eq)]
pub struct RawTaskName {
    name: String,
}

impl<T> From<T> for RawTaskName
where
    T: Display,
{
    fn from(value: T) -> Self {
        Self {
            name: value.to_string(),
        }
    }
}

impl TaskNameProvider for RawTaskName {
    fn raw(&self) -> String {
        self.name.clone()
    }
}

impl std::fmt::Debug for RawTaskName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskName (name={})", self.name)
    }
}
