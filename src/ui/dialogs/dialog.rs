// SPDX-License-Identifier: MIT

use crate::ui::widgets::WidgetTrait;

use async_trait::async_trait;
use std::any::Any;

#[async_trait]
pub trait DialogTrait: WidgetTrait {
    fn should_be_closed(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
}
