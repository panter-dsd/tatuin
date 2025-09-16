// SPDX-License-Identifier: MIT

use chrono::NaiveTime;

use crate::task::DateTimeUtc;

pub fn clear_time(dt: &DateTimeUtc) -> DateTimeUtc {
    const NULL_TIME: NaiveTime = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    dt.with_time(NULL_TIME).unwrap()
}

pub fn add_days(dt: &DateTimeUtc, days: u64) -> DateTimeUtc {
    dt.checked_add_days(chrono::Days::new(days)).unwrap()
}
