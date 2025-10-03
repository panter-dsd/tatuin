// SPDX-License-Identifier: MIT

pub fn exists(s: &str) -> bool {
    indent_chars().iter().any(|c| s.starts_with(*c))
}

pub fn indent_chars() -> [char; 2] {
    [' ', '\t']
}

pub fn is_indent(c: &char) -> bool {
    indent_chars().iter().any(|ch| c == ch)
}

pub fn trim_str(s: &str) -> &str {
    s.trim_start_matches(indent_chars())
}
