// SPDX-License-Identifier: MIT

use crate::ui::widgets::WidgetTrait;

use async_trait::async_trait;
use std::any::Any;

#[async_trait]
#[allow(dead_code)]
pub trait DialogTrait: WidgetTrait {
    fn accepted(&self) -> bool;
    fn rejected(&self) -> bool {
        !self.accepted()
    }
    fn should_be_closed(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
}
