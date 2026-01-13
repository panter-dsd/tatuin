// SPDX-License-Identifier: MIT

use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};

use crate::task::{DateTimeUtc, Priority, State, Task as TaskTrait, datetime_to_str};
use crate::time::{add_days, clear_time};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DatePatchItem {
    Today,
    Tomorrow,
    ThisWeekend,
    NextWeek,
    NoDate,
    Custom(DateTimeUtc),
}

impl std::fmt::Display for DatePatchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatePatchItem::Today => write!(f, "Today"),
            DatePatchItem::Tomorrow => write!(f, "Tomorrow"),
            DatePatchItem::ThisWeekend => write!(f, "This weekend"),
            DatePatchItem::NextWeek => write!(f, "Next week (Monday)"),
            DatePatchItem::NoDate => write!(f, "No date"),
            DatePatchItem::Custom(d) => {
                if d == &DateTimeUtc::default() {
                    write!(f, "Custom")
                } else {
                    let tz = Local::now().timezone();
                    write!(f, "Custom ({})", datetime_to_str(Some(*d), &tz))
                }
            }
        }
    }
}

impl DatePatchItem {
    fn to_date(self, current_dt: &DateTimeUtc) -> Option<DateTimeUtc> {
        let result = match self {
            DatePatchItem::Today => Some(*current_dt),
            DatePatchItem::Tomorrow => Some(add_days(current_dt, 1)),
            DatePatchItem::ThisWeekend => match current_dt.weekday() {
                chrono::Weekday::Sat | chrono::Weekday::Sun => Some(*current_dt),
                wd => Some(add_days(current_dt, 5 - wd as u64)),
            },
            DatePatchItem::NextWeek => Some(add_days(current_dt, 7 - current_dt.weekday() as u64)),
            DatePatchItem::NoDate => None,
            DatePatchItem::Custom(dt) => Some(dt),
        };

        result.map(|d| clear_time(&d))
    }

    pub fn values() -> Vec<DatePatchItem> {
        vec![
            DatePatchItem::Today,
            DatePatchItem::Tomorrow,
            DatePatchItem::ThisWeekend,
            DatePatchItem::NextWeek,
            DatePatchItem::NoDate,
        ]
    }
}

impl From<DatePatchItem> for Option<DateTimeUtc> {
    fn from(value: DatePatchItem) -> Option<DateTimeUtc> {
        value.to_date(&chrono::Utc::now())
    }
}

impl From<DateTimeUtc> for DatePatchItem {
    fn from(dt: DateTimeUtc) -> Self {
        let now = clear_time(&chrono::Utc::now());
        match (dt - now).num_days() {
            0 => DatePatchItem::Today,
            1 => DatePatchItem::Tomorrow,
            _ => DatePatchItem::Custom(dt),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ValuePatch<T> {
    #[default]
    NotSet,
    Empty,
    Value(T),
}

impl<T> From<Option<T>> for ValuePatch<T> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => Self::Value(v),
            None => Self::NotSet,
        }
    }
}

impl<T> ValuePatch<T>
where
    T: Clone,
{
    pub fn value(&self) -> Option<T> {
        match self {
            ValuePatch::NotSet | ValuePatch::Empty => None,
            ValuePatch::Value(v) => Some(v.clone()),
        }
    }

    pub fn ref_value(&self) -> Option<&T> {
        match self {
            ValuePatch::NotSet | ValuePatch::Empty => None,
            ValuePatch::Value(v) => Some(v),
        }
    }

    pub fn is_set(&self) -> bool {
        !matches!(self, ValuePatch::NotSet)
    }

    pub fn map<U, F>(self, f: F) -> ValuePatch<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ValuePatch::NotSet => ValuePatch::NotSet,
            ValuePatch::Empty => ValuePatch::Empty,
            ValuePatch::Value(x) => ValuePatch::Value(f(x)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::ValuePatch;

    #[test]
    fn value_patch_is_set() {
        type VP = ValuePatch<String>;
        assert!(!VP::NotSet.is_set());
        assert!(VP::Empty.is_set());
        assert!(VP::Value("some".to_string()).is_set());
    }
}

#[derive(Default)]
pub struct TaskPatch {
    pub task: Option<Box<dyn TaskTrait>>,
    pub name: ValuePatch<String>,
    pub description: ValuePatch<String>,
    pub due: ValuePatch<DatePatchItem>,
    pub scheduled: ValuePatch<DatePatchItem>,
    pub priority: ValuePatch<Priority>,
    pub state: ValuePatch<State>,
}

impl std::fmt::Display for TaskPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "TaskPatch {{ task_id: {}, task_title: {} state: {:?}, due: {:?}, scheduled: {:?}, priority: {:?}, name: {:?}, description: {:?}",
            self.task.as_ref().map(|t| t.id()).unwrap_or("-".to_string()),
            self.task.as_ref().map(|t| t.name().display()).unwrap_or("-".to_string()),
            self.state,
            self.due,
            self.scheduled,
            self.priority,
            self.name,
            self.description,
        ))
    }
}

impl std::fmt::Debug for TaskPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        !(self.name.is_set()
            || self.description.is_set()
            || self.due.is_set()
            || self.scheduled.is_set()
            || self.priority.is_set()
            || self.state.is_set())
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
            due: self.due.clone(),
            scheduled: self.scheduled.clone(),
            priority: self.priority.clone(),
            state: self.state.clone(),
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

impl Clone for PatchError {
    fn clone(&self) -> Self {
        Self {
            task: self.task.clone_boxed(),
            error: self.error.clone(),
        }
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
            due: DatePatchItem,
            now: DateTimeUtc,
            result: Option<DateTimeUtc>,
        }
        let cases: &[Case] = &[
            Case {
                name: "no date",
                due: DatePatchItem::NoDate,
                now: clear_time(&chrono::Utc::now()),
                result: None,
            },
            Case {
                name: "today",
                due: DatePatchItem::Today,
                now: clear_time(&chrono::Utc::now()),
                result: Some(clear_time(&chrono::Utc::now())),
            },
            Case {
                name: "tomorrow",
                due: DatePatchItem::Tomorrow,
                now: dt_from_unixtime(1749254400),
                result: Some(dt_from_unixtime(1749340800)),
            },
            Case {
                name: "this weekend for Monday",
                due: DatePatchItem::ThisWeekend,
                now: dt_from_unixtime(1748822400),
                result: Some(dt_from_unixtime(1749254400)),
            },
            Case {
                name: "this weekend for Friday",
                due: DatePatchItem::ThisWeekend,
                now: dt_from_unixtime(1749168000),
                result: Some(dt_from_unixtime(1749254400)),
            },
            Case {
                name: "this weekend for Saturday",
                due: DatePatchItem::ThisWeekend,
                now: dt_from_unixtime(1749254400),
                result: Some(dt_from_unixtime(1749254400)),
            },
            Case {
                name: "this weekend for Sunday",
                due: DatePatchItem::ThisWeekend,
                now: dt_from_unixtime(1749340800),
                result: Some(dt_from_unixtime(1749340800)),
            },
            Case {
                name: "next week for Monday",
                due: DatePatchItem::NextWeek,
                now: dt_from_unixtime(1748822400),
                result: Some(dt_from_unixtime(1749427200)),
            },
            Case {
                name: "next week for Friday",
                due: DatePatchItem::NextWeek,
                now: dt_from_unixtime(1749168000),
                result: Some(dt_from_unixtime(1749427200)),
            },
            Case {
                name: "next week for Saturday",
                due: DatePatchItem::NextWeek,
                now: dt_from_unixtime(1749254400),
                result: Some(dt_from_unixtime(1749427200)),
            },
            Case {
                name: "next week for Sunday",
                due: DatePatchItem::NextWeek,
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
