//! Shared page chrome: escaping and the page frame every route renders
//! through.

const STYLE: &str = include_str!("../../../assets/style.css");

pub(super) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

pub(super) fn forbidden_page() -> String {
    simple_page(
        "Forbidden",
        "<header class=\"lead\"><h1>This link is not valid</h1><p class=\"summary\">The server prints a new link every time it starts. Use the one from the terminal that started it.</p></header>",
    )
}

pub(super) fn simple_page(title: &str, body: &str) -> String {
    page(title, "", body, false)
}

pub(super) fn page(title: &str, current: &str, body: &str, wide: bool) -> String {
    let nav = |href: &str, label: &str| {
        if href == current {
            format!("<a href=\"{}\" aria-current=\"page\">{}</a>", href, label)
        } else {
            format!("<a href=\"{}\">{}</a>", href, label)
        }
    };
    let wide_class = if wide { " class=\"wide\"" } else { "" };
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{title}</title><style>{style}</style></head><body{wide_class}>\n<nav class=\"bar\"><span class=\"mark\">adr</span>{home}{next}{terms}</nav>\n<main{wide_class}>\n{body}</main>\n</body></html>\n",
        title = esc(title),
        style = STYLE,
        wide_class = wide_class,
        home = nav("/", "adr show"),
        next = nav("/next", "asked"),
        terms = nav("/terms", "adrdom/terms.md"),
        body = body
    )
}
