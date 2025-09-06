// SPDX-License-Identifier: MIT

use crate::task::Priority;

pub type TaskPriority = u8;

impl From<TaskPriority> for Priority {
    fn from(value: TaskPriority) -> Self {
        match value {
            0 | 5 => Priority::Normal,
            1 => Priority::Highest,
            2 | 3 => Priority::High,
            4 => Priority::Medium,
            6 | 7 => Priority::Low,
            8.. => Priority::Lowest,
        }
    }
}

impl From<Priority> for TaskPriority {
    fn from(value: Priority) -> Self {
        match value {
            Priority::Lowest => 8,
            Priority::Low => 6,
            Priority::Normal => 0,
            Priority::Medium => 4,
            Priority::High => 3,
            Priority::Highest => 1,
        }
    }
}
