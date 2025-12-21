use crate::RichStringTransformerTrait;
use regex::Regex;
use std::sync::LazyLock;

static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+://[^\s$)]+").unwrap());

#[derive(Debug)]
pub struct RawLinkTransformer {}

impl RichStringTransformerTrait for RawLinkTransformer {
    fn transform(&self, s: &str) -> String {
        fix_raw_links(s)
    }
}

fn fix_raw_links(text: &str) -> String {
    let mut result = String::new();

    let mut last_end: usize = 0;

    for m in URL_RE.find_iter(text) {
        let start = m.start();
        if start > 3 /*^[](http://) case*/ && text.get(start-2..start).is_some_and(|s| s == "](") {
            // detect []() link
            continue;
        }
        let mut s = m.as_str();
        let mut end = m.end();
        // check correctness of the url because the regexp is very simple
        if url::Url::parse(s).is_err() {
            // try one more time without the last symbol
            if url::Url::parse(&s[..s.len() - 1]).is_ok() {
                end -= 1;
                s = &s[..s.len() - 1];
            } else {
                continue;
            }
        }

        result.push_str(&text[last_end..start]);
        result.push('[');
        result.push_str(s);
        result.push_str("](");
        result.push_str(s);
        result.push(')');
        last_end = end;
    }
    result.push_str(&text[last_end..]);
    result
}

#[cfg(test)]
mod test {
    use super::fix_raw_links;

    #[test]
    fn fix_raw_links_test() {
        const FIXTURE:&str = "
obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81
[link](obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81)
Some http://some another http://hello/: and one more (http://yeeeee)
- [ ] Some task http://yandex.ru/some/uri 📅 2025-09-18
    First line https://login:password@host:1243/path/inside/file.txt: here
    Second line [link](http://ya.ru)
    Third line
";
        const RESULT:&str = "
[obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81](obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81)
[link](obsidian://open?vault=personal&file=%D0%92%D0%B5%D1%80%D0%B8%D0%BD%D0%B0%20%D0%BA%D1%80%D0%B0%D1%81%D0%BA%D0%B0%20%D0%B4%D0%BB%D1%8F%20%D0%B2%D0%BE%D0%BB%D0%BE%D1%81)
Some [http://some](http://some) another [http://hello/:](http://hello/:) and one more ([http://yeeeee](http://yeeeee))
- [ ] Some task [http://yandex.ru/some/uri](http://yandex.ru/some/uri) 📅 2025-09-18
    First line [https://login:password@host:1243/path/inside/file.txt:](https://login:password@host:1243/path/inside/file.txt:) here
    Second line [link](http://ya.ru)
    Third line
";

        let t = fix_raw_links(FIXTURE);
        assert_eq!(RESULT, t);
    }
}
