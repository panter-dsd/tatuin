// SPDX-License-Identifier: MIT

use tatuin_core::task::Priority;

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskPriority(u8);

impl TaskPriority {
    pub fn new(v: u8) -> Self {
        TaskPriority(v)
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskPriority{}", self.0)
    }
}

impl From<TaskPriority> for Priority {
    fn from(value: TaskPriority) -> Self {
        match value.0 {
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
            Priority::Lowest => TaskPriority::new(8),
            Priority::Low => TaskPriority::new(6),
            Priority::Normal => TaskPriority::new(0),
            Priority::Medium => TaskPriority::new(4),
            Priority::High => TaskPriority::new(3),
            Priority::Highest => TaskPriority::new(1),
        }
    }
}
