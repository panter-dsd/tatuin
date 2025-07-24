// SPDX-License-Identifier: MIT

use chrono::Datelike;

use crate::task::{DateTimeUtc, Priority, State, Task as TaskTrait};
use crate::time::{add_days, clear_time};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DuePatchItem {
    Today,
    Tomorrow,
    ThisWeekend,
    NextWeek,
    NoDate,
    Custom(DateTimeUtc),
}

impl DuePatchItem {
    pub fn to_date(self, dt: &DateTimeUtc) -> Option<DateTimeUtc> {
        let result = match self {
            DuePatchItem::Today => Some(*dt),
            DuePatchItem::Tomorrow => Some(add_days(dt, 1)),
            DuePatchItem::ThisWeekend => match dt.weekday() {
                chrono::Weekday::Sat | chrono::Weekday::Sun => Some(*dt),
                wd => Some(add_days(dt, 5 - wd as u64)),
            },
            DuePatchItem::NextWeek => Some(add_days(dt, 7 - dt.weekday() as u64)),
            DuePatchItem::NoDate => None,
            DuePatchItem::Custom(dt) => Some(dt),
        };

        result.map(|d| clear_time(&d))
    }

    pub fn values() -> Vec<DuePatchItem> {
        vec![
            DuePatchItem::Today,
            DuePatchItem::Tomorrow,
            DuePatchItem::ThisWeekend,
            DuePatchItem::NextWeek,
            DuePatchItem::NoDate,
        ]
    }
}

pub struct TaskPatch {
    pub task: Option<Box<dyn TaskTrait>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub due: Option<DuePatchItem>,
    pub priority: Option<Priority>,
    pub state: Option<State>,
}

impl std::fmt::Display for TaskPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "TaskPatch {{ task_id: {}, task_title: {} state: {:?}, due: {:?}, priority: {:?}, name: {:?}, description: {:?}",
            self.task.as_ref().map(|t| t.id()).unwrap_or("-".to_string()),
            self.task.as_ref().map(|t| t.text()).unwrap_or("-".to_string()),
            self.state,
            self.due,
            self.priority,
            self.name,
            self.description,
        ))
    }
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        self.state.is_none() && self.due.is_none() && self.priority.is_none()
    }

    pub fn is_task(&self, task: &dyn TaskTrait) -> bool {
        self.task
            .as_ref()
            .is_some_and(|t| t.id() == task.id() && t.provider() == task.provider())
    }
}

impl Clone for TaskPatch {
    fn clone(&self) -> Self {
        Self {
            task: if self.task.is_some() {
                Some(self.task.as_ref().unwrap().clone_boxed())
            } else {
                None
            },
            name: self.name.clone(),
            description: self.description.clone(),
            due: self.due,
            priority: self.priority,
            state: self.state,
        }
    }
}

pub struct PatchError {
    pub task: Box<dyn TaskTrait>,
    pub error: String,
}

impl PatchError {
    pub fn is_task(&self, task: &dyn TaskTrait) -> bool {
        self.task.id() == task.id() && self.task.provider() == task.provider()
    }
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error patching task with id {}: {}", self.task.id(), self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt_from_unixtime(secs: i64) -> DateTimeUtc {
        clear_time(&DateTimeUtc::from_timestamp(secs, 0).unwrap())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_patch_due_to_date() {
        struct Case<'a> {
            name: &'a str,
            due: DuePatchItem,
            now: DateTimeUtc,
            result: Option<DateTimeUtc>,
        }
        let cases: &[Case] = &[
            Case {
                name: "no date",
                due: DuePatchItem::NoDate,
                now: clear_time(&chrono::Utc::now()),
                result: None,
            },
            Case {
                name: "today",
                due: DuePatchItem::Today,
                now: clear_time(&chrono::Utc::now()),
                result: Some(clear_time(&chrono::Utc::now())),
            },
            Case {
                name: "tomorrow",
                due: DuePatchItem::Tomorrow,
                now: dt_from_unixtime(1749254400),
                result: Some(dt_from_unixtime(1749340800)),
            },
            Case {
                name: "this weekend for Monday",
                due: DuePatchItem::ThisWeekend,
                now: dt_from_unixtime(1748822400),
                result: Some(dt_from_unixtime(1749254400)),
            },
            Case {
                name: "this weekend for Friday",
                due: DuePatchItem::ThisWeekend,
                now: dt_from_unixtime(1749168000),
                result: Some(dt_from_unixtime(1749254400)),
            },
            Case {
                name: "this weekend for Saturday",
                due: DuePatchItem::ThisWeekend,
                now: dt_from_unixtime(1749254400),
                result: Some(dt_from_unixtime(1749254400)),
            },
            Case {
                name: "this weekend for Sunday",
                due: DuePatchItem::ThisWeekend,
                now: dt_from_unixtime(1749340800),
                result: Some(dt_from_unixtime(1749340800)),
            },
            Case {
                name: "next week for Monday",
                due: DuePatchItem::NextWeek,
                now: dt_from_unixtime(1748822400),
                result: Some(dt_from_unixtime(1749427200)),
            },
            Case {
                name: "next week for Friday",
                due: DuePatchItem::NextWeek,
                now: dt_from_unixtime(1749168000),
                result: Some(dt_from_unixtime(1749427200)),
            },
            Case {
                name: "next week for Saturday",
                due: DuePatchItem::NextWeek,
                now: dt_from_unixtime(1749254400),
                result: Some(dt_from_unixtime(1749427200)),
            },
            Case {
                name: "next week for Sunday",
                due: DuePatchItem::NextWeek,
                now: dt_from_unixtime(1749340800),
                result: Some(dt_from_unixtime(1749427200)),
            },
        ];

        for c in cases {
            let result = c.due.to_date(&c.now);
            assert_eq!(result, c.result, "Test '{}' was failed", c.name);
        }
    }
}
