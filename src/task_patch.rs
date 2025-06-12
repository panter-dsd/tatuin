use chrono::{Datelike, NaiveTime};

use crate::task::{DateTimeUtc, Priority, State, Task as TaskTrait};

#[derive(Clone)]
pub enum DuePatchItem {
    Today,
    Tomorrow,
    ThisWeekend,
    NextWeek,
    NoDate,
}

fn clear_time(dt: &DateTimeUtc) -> DateTimeUtc {
    dt.with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()).unwrap()
}

fn add_days(dt: &DateTimeUtc, days: u64) -> DateTimeUtc {
    dt.checked_add_days(chrono::Days::new(days)).unwrap()
}

impl DuePatchItem {
    pub fn to_date(&self, dt: &DateTimeUtc) -> Option<DateTimeUtc> {
        let result = match self {
            DuePatchItem::Today => Some(*dt),
            DuePatchItem::Tomorrow => Some(add_days(dt, 1)),
            DuePatchItem::ThisWeekend => match dt.weekday() {
                chrono::Weekday::Sat | chrono::Weekday::Sun => Some(*dt),
                wd => Some(add_days(dt, 5 - wd as u64)),
            },
            DuePatchItem::NextWeek => Some(add_days(dt, 7 - dt.weekday() as u64)),
            DuePatchItem::NoDate => None,
        };

        result.map(|d| clear_time(&d))
    }
}

pub struct TaskPatch {
    pub task: Box<dyn TaskTrait>,
    pub state: Option<State>,
    pub due: Option<DuePatchItem>,
    pub priority: Option<Priority>,
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        self.state.is_none() && self.due.is_none() && self.priority.is_none()
    }

    pub fn is_task(&self, task: &dyn TaskTrait) -> bool {
        self.task.id() == task.id() && self.task.provider() == task.provider()
    }
}

impl Clone for TaskPatch {
    fn clone(&self) -> Self {
        Self {
            task: self.task.clone_boxed(),
            state: self.state.clone(),
            due: self.due.clone(),
            priority: self.priority.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_patch_due_to_date() {
        struct Case<'a> {
            name: &'a str,
            due: DuePatchItem,
            now: DateTimeUtc,
            result: DateTimeUtc,
        }
        let cases: &[Case] = &[
            Case {
                name: "no date",
                due: DuePatchItem::NoDate,
                now: clear_time(&chrono::Utc::now()),
                result: DateTimeUtc::default(),
            },
            Case {
                name: "today",
                due: DuePatchItem::Today,
                now: clear_time(&chrono::Utc::now()),
                result: clear_time(&chrono::Utc::now()),
            },
            Case {
                name: "tomorrow",
                due: DuePatchItem::Tomorrow,
                now: clear_time(&DateTimeUtc::from_timestamp(1749254400, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749340800, 0).unwrap()),
            },
            Case {
                name: "this weekend for Monday",
                due: DuePatchItem::ThisWeekend,
                now: clear_time(&DateTimeUtc::from_timestamp(1748822400, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749254400, 0).unwrap()),
            },
            Case {
                name: "this weekend for Friday",
                due: DuePatchItem::ThisWeekend,
                now: clear_time(&DateTimeUtc::from_timestamp(1749168000, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749254400, 0).unwrap()),
            },
            Case {
                name: "this weekend for Saturday",
                due: DuePatchItem::ThisWeekend,
                now: clear_time(&DateTimeUtc::from_timestamp(1749254400, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749254400, 0).unwrap()),
            },
            Case {
                name: "this weekend for Sunday",
                due: DuePatchItem::ThisWeekend,
                now: clear_time(&DateTimeUtc::from_timestamp(1749340800, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749340800, 0).unwrap()),
            },
            Case {
                name: "next week for Monday",
                due: DuePatchItem::NextWeek,
                now: clear_time(&DateTimeUtc::from_timestamp(1748822400, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749427200, 0).unwrap()),
            },
            Case {
                name: "next week for Friday",
                due: DuePatchItem::NextWeek,
                now: clear_time(&DateTimeUtc::from_timestamp(1749168000, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749427200, 0).unwrap()),
            },
            Case {
                name: "next week for Saturday",
                due: DuePatchItem::NextWeek,
                now: clear_time(&DateTimeUtc::from_timestamp(1749254400, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749427200, 0).unwrap()),
            },
            Case {
                name: "next week for Sunday",
                due: DuePatchItem::NextWeek,
                now: clear_time(&DateTimeUtc::from_timestamp(1749340800, 0).unwrap()),
                result: clear_time(&DateTimeUtc::from_timestamp(1749427200, 0).unwrap()),
            },
        ];

        for c in cases {
            let result = c.due.to_date(&c.now);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), c.result, "Test '{}' was failed", c.name);
        }
    }
}
