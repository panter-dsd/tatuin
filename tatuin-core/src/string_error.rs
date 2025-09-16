use std::error::Error;

#[derive(Debug, Clone)]
pub struct StringError {
    message: String,
}

impl StringError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl From<Box<dyn Error>> for StringError {
    fn from(e: Box<dyn Error>) -> Self {
        Self { message: e.to_string() }
    }
}

impl From<StringError> for Box<dyn Error> {
    fn from(e: StringError) -> Self {
        Box::<dyn Error>::from(e.message)
    }
}

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
