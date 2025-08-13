use crate::task::Task as TaskTrait;

#[derive(Default)]
pub struct Task {
    pub uid: String,
    pub name: String,
}

impl TaskTrait for Task {
    fn id(&self) -> String {
        todo!()
    }

    fn text(&self) -> String {
        todo!()
    }

    fn state(&self) -> crate::task::State {
        todo!()
    }

    fn provider(&self) -> String {
        todo!()
    }

    fn project(&self) -> Option<Box<dyn crate::project::Project>> {
        todo!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        todo!()
    }

    fn clone_boxed(&self) -> Box<dyn TaskTrait> {
        todo!()
    }
}
