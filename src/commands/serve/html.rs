//! Shared page chrome: escaping, pluralization, the banner, and the page
//! frame every route renders through.
use crate::out::{self, Line};

const STYLE: &str = include_str!("../../../assets/style.css");

/// The page's one escaper: `out::esc`, the one every line renders through
/// (a control or bidirectional character is shown written out).
pub(super) fn esc(s: &str) -> String {
    out::esc(s)
}

pub(super) fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {}", one)
    } else {
        format!("{} {}", n, many)
    }
}

/// The lines as the page renders them, each through `out::html`.
pub(super) fn lines(lines: &[Line]) -> String {
    out::present(lines).into_iter().map(|l| format!("{}\n", out::html(l))).collect()
}

/// A status line as small badges: the status name (the version of an asked
/// question left off unless `version`) and the subject, which is `link`ed
/// when an href is given. Same words as the line `out::text` prints.
pub(super) fn st(line: &Line, version: bool, href: Option<&str>) -> String {
    let Line::Status { subject, status, .. } = line else { return out::html(line) };
    let mut name = status.name();
    if !version {
        if let Some((n, _)) = name.split_once(" @") {
            name = n.to_string();
        }
    }
    let class = name.split(' ').next().unwrap_or("").to_string();
    let subject = match href {
        Some(h) => format!("<a href=\"{}\">{}</a>", h, esc(subject)),
        None => esc(subject),
    };
    format!(
        "<span class=\"st\"><span class=\"badge b-{}\">{}</span> <span class=\"subject\">{}</span></span>",
        esc(&class),
        esc(&name),
        subject
    )
}

/// What the last act printed (its events, and the findings on its input or
/// that stopped it), shown once above the page it leads to. Each line keeps
/// its own kind; the banner reads as a stop only when the act wrote nothing.
pub(super) fn banner(last: &[Line]) -> String {
    if last.is_empty() {
        return String::new();
    }
    let stopped = !last.iter().any(|l| matches!(l, Line::Event(_)));
    format!("<div class=\"banner{}\">\n{}</div>\n", if stopped { " refused" } else { "" }, lines(last))
}

/// The neighbour lines of a question, folded under a summary that counts
/// them; the line that says no records exist stays outside the fold.
pub(super) fn neighbours(lines: &[String]) -> String {
    let none = crate::msg::show_no_records();
    let (plain, folded): (Vec<&str>, Vec<&str>) = lines.iter().map(|l| l.trim()).partition(|l| *l == none);
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
