// SPDX-License-Identifier: MIT

use crate::project::Project as ProjectTrait;

#[derive(Clone, Debug, Default)]
pub struct Project {}

impl ProjectTrait for Project {
    fn id(&self) -> String {
        "default".to_string()
    }

    fn name(&self) -> String {
        "Default".to_string()
    }

    fn provider(&self) -> String {
        super::PROVIDER_NAME.to_string()
    }

    fn description(&self) -> String {
        "Default project".to_string()
    }

    fn parent_id(&self) -> Option<String> {
        None
    }

    fn is_inbox(&self) -> bool {
        true
    }

    fn is_favorite(&self) -> bool {
        true
    }

    fn clone_boxed(&self) -> Box<dyn ProjectTrait> {
        Box::new(self.clone())
    }
}
