//! The markdown subset vow posts may use (`sprints/sprint09.md`), rendered
//! to HTML that is safe to put in a page: paragraphs, line breaks, `- `
//! lists, `**strong**`, `*emphasis*` (or `_emphasis_`), and
//! `[links](https://…)` to http(s) only. Everything else is text: every
//! `<`, `>`, `&` and quote is escaped, so no input can make markup. The
//! same function will feed Sprint 11's Mastodon mirror.

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

/// A link target we allow: http(s), no spaces or quotes.
fn safe_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && url.len() > 8
        && !url
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>'))
}

/// `[text](url)` at the start of `s`: (text, url, bytes used).
fn link_at(s: &str) -> Option<(&str, &str, usize)> {
    let rest = s.strip_prefix('[')?;
    let close = rest.find("](")?;
    let text = &rest[..close];
    if text.is_empty() || text.contains('[') {
        return None;
    }
    let after = &rest[close + 2..];
    let end = after.find(')')?;
    let url = &after[..end];
    safe_url(url).then_some((text, url, 1 + close + 2 + end + 1))
}

/// Inline markup in one line (or link text).
fn inline(s: &str, out: &mut String) {
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if let Some((text, url, used)) = link_at(rest) {
            out.push_str("<a href=\"");
            escape(url, out);
            out.push_str("\" rel=\"nofollow noopener noreferrer\" target=\"_blank\">");
            inline(text, out);
            out.push_str("</a>");
            i += used;
            continue;
        }
        if let Some(inner) = rest.strip_prefix("**") {
            if let Some(end) = inner.find("**").filter(|&e| e > 0) {
                out.push_str("<strong>");
                inline(&inner[..end], out);
                out.push_str("</strong>");
                i += 2 + end + 2;
                continue;
            }
        }
        let emphasis = ['*', '_'].into_iter().find_map(|mark| {
            let inner = rest.strip_prefix(mark)?;
            let end = inner.find(mark).filter(|&e| e > 0)?;
            (!inner[..end].starts_with(' ')).then_some(&inner[..end])
        });
        if let Some(inner) = emphasis {
            out.push_str("<em>");
            inline(inner, out);
            out.push_str("</em>");
            i += inner.len() + 2;
            continue;
        }
        let c = rest.chars().next().expect("not empty");
        escape(c.encode_utf8(&mut [0; 4]), out);
        i += c.len_utf8();
    }
}

fn list_item(line: &str) -> Option<&str> {
    line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))
}

/// `md` as safe HTML.
pub fn to_html(md: &str) -> String {
    let mut out = String::new();
    let text = md.replace("\r\n", "\n");
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.trim().is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        if lines.iter().all(|l| list_item(l.trim_start()).is_some()) {
            out.push_str("<ul>");
            for l in lines {
                out.push_str("<li>");
                inline(list_item(l.trim_start()).unwrap_or(l), &mut out);
                out.push_str("</li>");
            }
            out.push_str("</ul>");
            continue;
        }
        out.push_str("<p>");
        for (n, l) in lines.iter().enumerate() {
            if n > 0 {
                out.push_str("<br>");
            }
            inline(l.trim(), &mut out);
        }
        out.push_str("</p>");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_html;

    #[test]
    fn the_subset() {
        assert_eq!(
            to_html("Hello\nworld\n\nAgain"),
            "<p>Hello<br>world</p><p>Again</p>"
        );
        assert_eq!(
            to_html("**Kept** it *today*, _gently_."),
            "<p><strong>Kept</strong> it <em>today</em>, <em>gently</em>.</p>"
        );
        assert_eq!(to_html("- one\n- two"), "<ul><li>one</li><li>two</li></ul>");
        assert_eq!(
            to_html("See [CBETA](https://cbetaonline.dila.edu.tw/zh/T1484)"),
            "<p>See <a href=\"https://cbetaonline.dila.edu.tw/zh/T1484\" rel=\"nofollow noopener noreferrer\" target=\"_blank\">CBETA</a></p>"
        );
        assert_eq!(to_html("2 * 3 * 4"), "<p>2 * 3 * 4</p>");
        assert_eq!(to_html("「不殺生」"), "<p>「不殺生」</p>");
    }

    #[test]
    fn hostile_input_stays_text() {
        let cases = [
            "<script>alert(1)</script>",
            "<img src=x onerror=alert(1)>",
            "[click](javascript:alert(1))",
            "[click](JaVaScRiPt:alert(1))",
            "[x](https://a.example/\"onmouseover=\"alert(1))",
            "[x](data:text/html,<script>alert(1)</script>)",
            "**<b>bold</b>**",
            "*<iframe src=//evil>*",
            "[<svg onload=alert(1)>](https://ok.example)",
            "&lt;script&gt;",
            "<a href=\"https://x\">x</a>",
        ];
        for md in cases {
            let html = to_html(md);
            let tags: Vec<&str> = html
                .split('<')
                .skip(1)
                .map(|t| t.split(['>', ' ']).next().unwrap_or(""))
                .collect();
            assert!(
                tags.iter().all(|t| matches!(
                    *t,
                    "p" | "/p"
                        | "br"
                        | "strong"
                        | "/strong"
                        | "em"
                        | "/em"
                        | "a"
                        | "/a"
                        | "ul"
                        | "/ul"
                        | "li"
                        | "/li"
                )),
                "{md:?} -> {html}"
            );
            // Hostile text may remain, as text; only http(s) is ever a link.
            for (i, _) in html.match_indices("href=\"") {
                let url = &html[i + 6..];
                let url = &url[..url.find('"').unwrap()];
                assert!(url.starts_with("http"), "{md:?} -> {html}");
            }
        }
        assert_eq!(to_html("&lt;"), "<p>&amp;lt;</p>");
    }
}
