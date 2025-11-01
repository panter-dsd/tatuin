// SPDX-License-Identifier: MIT

use std::sync::LazyLock;

use regex::Regex;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct LinkSearchResult<'a> {
    pub start: usize,
    pub end: usize,
    heading_separator_pos: Option<usize>,
    display_text_separator_pos: Option<usize>,
    pub display_text: &'a str,
    pub link: &'a str,
}

pub fn find_wiki_links<'a>(text: &'a str) -> Vec<LinkSearchResult<'a>> {
    let mut result = Vec::new();

    let mut prev_char = '_';
    let mut link: Option<LinkSearchResult> = None;

    for (pos, c) in text.char_indices() {
        if c == '[' && prev_char == '[' {
            link = Some(LinkSearchResult {
                start: pos - 1,
                end: pos,
                heading_separator_pos: None,
                display_text_separator_pos: None,
                display_text: &text[text.len()..],
                link: &text[pos + 1..],
            });
        } else if c == ']'
            && prev_char == ']'
            && let Some(l) = &mut link
        {
            l.end = pos;
            l.link = &text[l.start + 2
                ..l.heading_separator_pos
                    .unwrap_or(l.display_text_separator_pos.unwrap_or(l.end - 1))];
            if let Some(pos) = l.display_text_separator_pos {
                l.display_text = &text[pos + 1..l.end - 1];
            }
            result.push(*l);
            link = None;
        } else if c == '|'
            && let Some(l) = &mut link
        {
            if l.display_text_separator_pos.is_some() {
                link = None;
            } else {
                l.display_text_separator_pos = Some(pos);
            }
        } else if c == '#'
            && let Some(l) = &mut link
        {
            if l.heading_separator_pos.is_some() {
                link = None;
            } else {
                l.heading_separator_pos = Some(pos);
            }
        }

        prev_char = c;
    }

    result
}

pub fn find_regular_links<'a>(text: &'a str) -> Vec<LinkSearchResult<'a>> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+.md)\)").unwrap());

    let mut result = Vec::new();

    for cap in RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let (_, [display_text, link]) = cap.extract();

        result.push(LinkSearchResult {
            start: m.start(),
            end: m.end(),
            heading_separator_pos: None,
            display_text_separator_pos: None,
            display_text,
            link,
        });
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_wiki_links_empty_string() {
        let result = find_wiki_links("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_wiki_links_regular_string_without_any_link() {
        let result = find_wiki_links("some string");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_wiki_links_single_simple_link_alone() {
        let result = find_wiki_links("[[some string]]");
        let expected = vec![LinkSearchResult {
            start: 0,
            end: 14,
            heading_separator_pos: None,
            display_text_separator_pos: None,
            display_text: "",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_simple_link_with_suffix() {
        let result = find_wiki_links("Text [[some string]]");
        let expected = vec![LinkSearchResult {
            start: 5,
            end: 19,
            heading_separator_pos: None,
            display_text_separator_pos: None,
            display_text: "",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_simple_link_with_prefix() {
        let result = find_wiki_links("[[some string]] text");
        let expected = vec![LinkSearchResult {
            start: 0,
            end: 14,
            heading_separator_pos: None,
            display_text_separator_pos: None,
            display_text: "",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_simple_link_with_prefix_and_suffix() {
        let result = find_wiki_links("Text [[some string]] text");
        let expected = vec![LinkSearchResult {
            start: 5,
            end: 19,
            heading_separator_pos: None,
            display_text_separator_pos: None,
            display_text: "",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_link_alone() {
        let result = find_wiki_links("[[some string|name]]");
        let expected = vec![LinkSearchResult {
            start: 0,
            end: 19,
            heading_separator_pos: None,
            display_text_separator_pos: Some(13),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_link_with_suffix() {
        let result = find_wiki_links("Text [[some string|name]]");
        let expected = vec![LinkSearchResult {
            start: 5,
            end: 24,
            heading_separator_pos: None,
            display_text_separator_pos: Some(18),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_link_with_prefix() {
        let result = find_wiki_links("[[some string|name]] text");
        let expected = vec![LinkSearchResult {
            start: 0,
            end: 19,
            heading_separator_pos: None,
            display_text_separator_pos: Some(13),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_link_with_prefix_and_suffix() {
        let result = find_wiki_links("Text [[some string|name]] text");
        let expected = vec![LinkSearchResult {
            start: 5,
            end: 24,
            heading_separator_pos: None,
            display_text_separator_pos: Some(18),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_and_heading_link_alone() {
        let result = find_wiki_links("[[some string#heading|name]]");
        let expected = vec![LinkSearchResult {
            start: 0,
            end: 27,
            heading_separator_pos: Some(13),
            display_text_separator_pos: Some(21),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_and_heading_link_with_suffix() {
        let result = find_wiki_links("Text [[some string#heading|name]]");
        let expected = vec![LinkSearchResult {
            start: 5,
            end: 32,
            heading_separator_pos: Some(18),
            display_text_separator_pos: Some(26),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_and_heading_link_with_prefix() {
        let result = find_wiki_links("[[some string#heading|name]] text");
        let expected = vec![LinkSearchResult {
            start: 0,
            end: 27,
            heading_separator_pos: Some(13),
            display_text_separator_pos: Some(21),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_single_with_display_name_and_heading_link_with_prefix_and_suffix() {
        let result = find_wiki_links("Text [[some string#heading|name]] text");
        let expected = vec![LinkSearchResult {
            start: 5,
            end: 32,
            heading_separator_pos: Some(18),
            display_text_separator_pos: Some(26),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_several_links_in_one_text() {
        let result =
            find_wiki_links("Text [[some string#heading|name]] text [[another link]] [[and one more|link]] end");
        let expected = vec![
            LinkSearchResult {
                start: 5,
                end: 32,
                heading_separator_pos: Some(18),
                display_text_separator_pos: Some(26),
                display_text: "name",
                link: "some string",
            },
            LinkSearchResult {
                start: 39,
                end: 54,
                heading_separator_pos: None,
                display_text_separator_pos: None,
                display_text: "",
                link: "another link",
            },
            LinkSearchResult {
                start: 56,
                end: 76,
                heading_separator_pos: None,
                display_text_separator_pos: Some(70),
                display_text: "link",
                link: "and one more",
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_several_links_in_one_text_with_cyrillic() {
        let result = find_wiki_links(
            "Текст [[одна строка#ссылка внутри строки|имя]] текст [[другая ссылка]] [[и еще одна|ссылка]] конец",
        );
        let expected = vec![
            LinkSearchResult {
                start: 11,
                end: 81,
                heading_separator_pos: Some(34),
                display_text_separator_pos: Some(73),
                display_text: "имя",
                link: "одна строка",
            },
            LinkSearchResult {
                start: 94,
                end: 122,
                heading_separator_pos: None,
                display_text_separator_pos: None,
                display_text: "",
                link: "другая ссылка",
            },
            LinkSearchResult {
                start: 124,
                end: 158,
                heading_separator_pos: None,
                display_text_separator_pos: Some(144),
                display_text: "ссылка",
                link: "и еще одна",
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_unfinished_link_and_a_good_one() {
        let result = find_wiki_links("Text [[some [[some string#heading|name]] text");
        let expected = vec![LinkSearchResult {
            start: 12,
            end: 39,
            heading_separator_pos: Some(25),
            display_text_separator_pos: Some(33),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_nested_link() {
        let result = find_wiki_links("Text [[some [[some string#heading|name]] text]");
        let expected = vec![LinkSearchResult {
            start: 12,
            end: 39,
            heading_separator_pos: Some(25),
            display_text_separator_pos: Some(33),
            display_text: "name",
            link: "some string",
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_two_separators() {
        let result = find_wiki_links("Text [[some string#heading|name|name]] text");
        let expected = Vec::new();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_wiki_links_two_hash_separators() {
        let result = find_wiki_links("Text [[some string#heading#|name]] text");
        let expected = Vec::new();
        assert_eq!(result, expected);
    }
}
