use std::fmt::{Debug, Display};

pub trait Trait: Debug {
    fn raw(&self) -> String;
    fn display(&self) -> String {
        self.raw()
    }
}

#[derive(PartialEq, Eq)]
pub struct DefaultImpl {
    s: String,
}

impl<T> From<T> for DefaultImpl
where
    T: Display,
{
    fn from(value: T) -> Self {
        Self { s: value.to_string() }
    }
}

impl Trait for DefaultImpl {
    fn raw(&self) -> String {
        self.s.clone()
    }
}

impl std::fmt::Debug for DefaultImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RichString (s={})", self.s)
    }
}
