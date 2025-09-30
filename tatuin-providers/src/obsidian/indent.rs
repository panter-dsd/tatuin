pub struct Indent<'a> {
    s: &'a str,
}

impl<'a> Indent<'a> {
    pub fn new(s: &'a str) -> Self {
        Self { s }
    }

    pub fn exists(&self) -> bool {
        Self::chars().iter().any(|c| self.s.starts_with(*c))
    }

    pub fn chars() -> [char; 2] {
        [' ', '\t']
    }

    pub fn is_indent(c: &char) -> bool {
        Self::chars().iter().any(|ch| c == ch)
    }

    pub fn trim_str(s: &'a str) -> &'a str {
        s.trim_start_matches(Self::chars())
    }
}
