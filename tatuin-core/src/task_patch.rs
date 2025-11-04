// SPDX-License-Identifier: MIT

use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};

use crate::task::{DateTimeUtc, Priority, State, Task as TaskTrait, datetime_to_str};
use crate::time::{add_days, clear_time};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DuePatchItem {
    Today,
    Tomorrow,
    ThisWeekend,
    NextWeek,
    NoDate,
    Custom(DateTimeUtc),
}

impl std::fmt::Display for DuePatchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuePatchItem::Today => write!(f, "Today"),
            DuePatchItem::Tomorrow => write!(f, "Tomorrow"),
            DuePatchItem::ThisWeekend => write!(f, "This weekend"),
            DuePatchItem::NextWeek => write!(f, "Next week (Monday)"),
            DuePatchItem::NoDate => write!(f, "No date"),
            DuePatchItem::Custom(d) => {
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

impl DuePatchItem {
    fn to_date(self, current_dt: &DateTimeUtc) -> Option<DateTimeUtc> {
        let result = match self {
            DuePatchItem::Today => Some(*current_dt),
            DuePatchItem::Tomorrow => Some(add_days(current_dt, 1)),
            DuePatchItem::ThisWeekend => match current_dt.weekday() {
                chrono::Weekday::Sat | chrono::Weekday::Sun => Some(*current_dt),
                wd => Some(add_days(current_dt, 5 - wd as u64)),
            },
            DuePatchItem::NextWeek => Some(add_days(current_dt, 7 - current_dt.weekday() as u64)),
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

impl From<DuePatchItem> for Option<DateTimeUtc> {
    fn from(value: DuePatchItem) -> Option<DateTimeUtc> {
        value.to_date(&chrono::Utc::now())
    }
}

impl From<DateTimeUtc> for DuePatchItem {
    fn from(dt: DateTimeUtc) -> Self {
        let now = clear_time(&chrono::Utc::now());
        match (dt - now).num_days() {
            0 => DuePatchItem::Today,
            1 => DuePatchItem::Tomorrow,
            _ => DuePatchItem::Custom(dt),
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
    pub due: ValuePatch<DuePatchItem>,
    pub priority: ValuePatch<Priority>,
    pub state: ValuePatch<State>,
}

impl std::fmt::Display for TaskPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "TaskPatch {{ task_id: {}, task_title: {} state: {:?}, due: {:?}, priority: {:?}, name: {:?}, description: {:?}",
            self.task.as_ref().map(|t| t.id()).unwrap_or("-".to_string()),
            self.task.as_ref().map(|t| t.name().display()).unwrap_or("-".to_string()),
            self.state,
            self.due,
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
