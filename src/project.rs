// SPDX-License-Identifier: MIT

use std::fmt::Debug;

#[allow(dead_code)]
pub trait Project: Send + Sync + Debug {
    fn id(&self) -> String;
    fn name(&self) -> String;
    fn provider(&self) -> String;
    fn description(&self) -> String;
    fn parent_id(&self) -> Option<String>;
    fn is_inbox(&self) -> bool;
    fn is_favorite(&self) -> bool;
    fn clone_boxed(&self) -> Box<dyn Project>;
}
