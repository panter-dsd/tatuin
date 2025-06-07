// SPDX-License-Identifier: MIT

mod client;
mod md_file;
mod patch;
mod project;
mod task;

use crate::filter;
use crate::project::Project as ProjectTrait;
use crate::provider::{DuePatchItem, PatchError, Provider as ProviderTrait, TaskPatch};
use crate::task::{DateTimeUtc, Task as TaskTrait};
use async_trait::async_trait;
use chrono::{Datelike, NaiveTime};
use ratatui::style::Color;
use std::error::Error;

pub const PROVIDER_NAME: &str = "Obsidian";

pub struct Provider {
    name: String,
    c: client::Client,
    color: Color,
}

impl Provider {
    pub fn new(name: &str, path: &str, color: &Color) -> Self {
        Self {
            name: name.to_string(),
            c: client::Client::new(path),
            color: *color,
        }
    }
}

#[async_trait]
impl ProviderTrait for Provider {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn type_name(&self) -> String {
        PROVIDER_NAME.to_string()
    }

    async fn tasks(
        &mut self,
        _project: Option<Box<dyn ProjectTrait>>,
        f: &filter::Filter,
    ) -> Result<Vec<Box<dyn TaskTrait>>, Box<dyn Error>> {
        let tasks = self.c.tasks(f).await?;
        let mut result: Vec<Box<dyn TaskTrait>> = Vec::new();
        for mut t in tasks {
            t.set_provider(self.name());
            result.push(Box::new(t));
        }
        Ok(result)
    }

    async fn projects(&mut self) -> Result<Vec<Box<dyn ProjectTrait>>, Box<dyn Error>> {
        Ok(Vec::new())
    }

    async fn patch_tasks(&mut self, patches: &[TaskPatch]) -> Vec<PatchError> {
        let mut client_patches = Vec::new();
        let mut errors = Vec::new();
        let now = chrono::Utc::now();
        for p in patches.iter() {
            match p.task.as_any().downcast_ref::<task::Task>() {
                Some(t) => client_patches.push(patch::TaskPatch {
                    task: t,
                    state: p.state.clone().map(|s| s.into()),
                    due: p.due.clone().map(|due| patch_due_to_date(&now, &due)),
                }),
                None => panic!("Wrong casting!"),
            };
        }

        for e in self.c.patch_tasks(&client_patches).await {
            errors.push(PatchError {
                task: e.task.clone_boxed(),
                error: e.error,
            })
        }

        errors
    }

    async fn reload(&mut self) {
        // do nothing for now
    }

    fn color(&self) -> Color {
        self.color
    }
}

fn clear_time(dt: &DateTimeUtc) -> DateTimeUtc {
    dt.with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()).unwrap()
}

fn patch_due_to_date(now: &DateTimeUtc, due: &DuePatchItem) -> DateTimeUtc {
    let dt = match due {
        DuePatchItem::Today => now,
        DuePatchItem::Tomorrow => &now.checked_add_days(chrono::Days::new(1)).unwrap(),
        DuePatchItem::ThisWeekend => match now.weekday() {
            chrono::Weekday::Sat | chrono::Weekday::Sun => now,
            wd => &now.checked_add_days(chrono::Days::new(5 - wd as u64)).unwrap(),
        },
        DuePatchItem::NextWeek => &now
            .checked_add_days(chrono::Days::new(7 - now.weekday() as u64))
            .unwrap(),
        DuePatchItem::NoDate => &DateTimeUtc::default(),
    };

    clear_time(dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
            let result = patch_due_to_date(&c.now, &c.due);
            assert_eq!(result, c.result, "Test '{}' was failed", c.name);
        }
    }
}
