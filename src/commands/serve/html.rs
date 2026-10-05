//! Shared page chrome: escaping, pluralization, the banner, and the page
//! frame every route renders through.

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

pub(super) fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {}", one)
    } else {
        format!("{} {}", n, many)
    }
}

/// The neighbour lines of a question, folded under a summary that counts
/// them; a line that is not a neighbour stays outside the fold.
pub(super) fn neighbours_html(lines: &[String]) -> String {
    let (folded, plain): (Vec<&str>, Vec<&str>) = lines
        .iter()
        .map(|l| l.trim())
        .partition(|l| l.starts_with("neighbour: "));
    let mut out = String::new();
    if !folded.is_empty() {
        out.push_str(&format!(
            "<details class=\"neighbours\"><summary>{}</summary><ul>",
            plural(folded.len(), "neighbour", "neighbours")
        ));
        for l in &folded {
            out.push_str(&format!("<li>{}</li>", esc(l)));
        }
        out.push_str("</ul></details>\n");
    }
    if !plain.is_empty() {
        out.push_str("<ul class=\"neighbours\">");
        for l in &plain {
            out.push_str(&format!("<li>{}</li>", esc(l)));
        }
        out.push_str("</ul>\n");
    }
    out
}

pub(super) fn banner_html(m: Option<&String>) -> String {
    match m {
        Some(text) if !text.is_empty() => {
            let refused = text.contains("refusing") || text.contains("failed");
            format!(
                "<p class=\"banner{}\">{}</p>\n",
                if refused { " refused" } else { "" },
                esc(text)
            )
        }
        _ => String::new(),
    }
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
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{title}</title><style>{style}</style></head><body{wide_class}>\n<nav class=\"bar\"><span class=\"mark\">adr</span>{home}{next}{decisions}{terms}</nav>\n<main{wide_class}>\n{body}</main>\n</body></html>\n",
        title = esc(title),
        style = STYLE,
        wide_class = wide_class,
        home = nav("/", "Overview"),
        next = nav("/next", "Answer"),
        decisions = nav("/decisions", "Decisions"),
        terms = nav("/terms", "Terms"),
        body = body
    )
}
