//! One list, one matcher, for text that reports who said or chose
//! something, what the code does right now, when something was said, or
//! that opens a heading already presupposing an answer (per "Does the tool
//! stop loaded phrases in a question?"). `approve`
//! and `ask` refuse on a hit; `check` reports the same hits, but only in
//! text not yet frozen (a section whose latest ledger state is not
//! `answered`, `refused`, or `superseded`) plus the context prose, which is
//! never frozen.
//!
//! Used by `actions::do_approve_record` (a heading with any tell, or a
//! context prose line with a provenance/code-state/date tell, refuses
//! before any write), `commands::ask::run` (a candidate section's body with
//! a provenance/code-state/date tell refuses), and
//! `commands::check::tells` (the same scan, reported rather than refused,
//! for unfrozen text).

/// Who said, chose, or wanted what, and where or when it was said. No actor
/// word: the phrases hold in any repository, whatever it calls its people
/// and its agents.
pub const PROVENANCE: &[&str] = &[
    "said",
    "told",
    "agreed",
    "chose",
    "decided",
    "wants",
    "prefers",
    "in chat",
    "in the chat",
    "this session",
    "as agreed",
    "as discussed",
    "as decided",
    "claude",
    "codex",
    "gemini",
    "copilot",
];

/// What the description says the tool does or holds at the moment of
/// writing, or an option that only ratifies that.
pub const CODE_STATE: &[&str] = &[
    "already",
    "currently",
    "today",
    "right now",
    "at the moment",
    "as built",
    "as implemented",
    "as is",
    "as it is",
    "unchanged",
    "no change",
    "keep it",
    "nothing to build",
];

/// A heading opening with one of these presupposes the set, the place, or
/// the count it asks about, rather than whether the thing exists at all.
pub const LOADED_OPENINGS: &[&str] = &["which", "where", "how many", "how much"];

pub const PROVENANCE_WHAT: &str = "says who said it";
pub const CODE_STATE_WHAT: &str = "states what the code does now";
pub const DATE_WHAT: &str = "carries a date";
pub const LOADED_OPENING_WHAT: &str = "opens assuming an answer";
pub const JOIN_WHAT: &str = "joins two questions";

pub const ID_PREFIX: &str = "id: ";
pub const TERMS_PREFIX: &str = "terms: ";
pub const DEPENDS_PREFIX: &str = "depends: ";
pub const SUPERSEDES_PREFIX: &str = "supersedes: ";
pub const ANSWER_PREFIX: &str = "**Answer:**";

/// Every prefix a line the tool writes can start with. `is_tool_line`
/// checks all five; a caller that wants only some of them (e.g.
/// id/supersedes, or id/depends/supersedes without the terms line) calls
/// `starts_with_any` with its own subset of these constants instead.
pub const TOOL_LINE_PREFIXES: &[&str] = &[ID_PREFIX, TERMS_PREFIX, DEPENDS_PREFIX, SUPERSEDES_PREFIX, ANSWER_PREFIX];

/// True when `line`, trimmed of leading whitespace, starts with any of
/// `prefixes` (each an exact prefix, e.g. `tells::ID_PREFIX`).
pub fn starts_with_any(line: &str, prefixes: &[&str]) -> bool {
    let t = line.trim_start();
    prefixes.iter().any(|p| t.starts_with(p))
}

/// A line the tool writes, or that only points at other text by id: never
/// the agent's own prose, so never scanned for a tell.
pub fn is_tool_line(line: &str) -> bool {
    starts_with_any(line, TOOL_LINE_PREFIXES)
}

/// Lowercased runs of alphanumerics: the words of a line, so a tell matches
/// whole words only ("as is" never matches inside "has issues").
pub fn words(text: &str) -> Vec<String> {
    crate::text::word_runs(&text.to_lowercase())
}

/// Whether `tell`'s words occur contiguously in `line_words`.
fn has_phrase(line_words: &[String], tell: &str) -> bool {
    let tw = words(tell);
    !tw.is_empty() && line_words.windows(tw.len()).any(|w| w == tw.as_slice())
}

/// The first date of the form YYYY-MM-DD in `line`, if any.
pub fn find_date(line: &str) -> Option<&str> {
    let b = line.as_bytes();
    (0..b.len().saturating_sub(9)).find_map(|i| {
        let is_date = b[i..i + 4].iter().all(u8::is_ascii_digit)
            && b[i + 4] == b'-'
            && b[i + 5..i + 7].iter().all(u8::is_ascii_digit)
            && b[i + 7] == b'-'
            && b[i + 8..i + 10].iter().all(u8::is_ascii_digit);
        is_date.then(|| &line[i..i + 10])
    })
}

/// The provenance, code-state and date tells in one line of text, at
/// absolute line `i`: the one scan both prose and headings get.
fn line_tells(i: usize, text: &str) -> Vec<(usize, String, &'static str)> {
    let mut out = Vec::new();
    let lw = words(text);
    for tell in PROVENANCE {
        if has_phrase(&lw, tell) {
            out.push((i, (*tell).to_string(), PROVENANCE_WHAT));
        }
    }
    for tell in CODE_STATE {
        if has_phrase(&lw, tell) {
            out.push((i, (*tell).to_string(), CODE_STATE_WHAT));
        }
    }
    if let Some(d) = find_date(text) {
        out.push((i, d.to_string(), DATE_WHAT));
    }
    out
}

/// Every provenance, code-state, or date tell in `lines[start..end]`, tool
/// lines skipped: (absolute 0-based line index, the matched phrase or date,
/// what it does). Used for a section's body (`ask`, `check`) and for the
/// context prose (`approve`, `check`).
pub fn prose_tells_in(lines: &[String], start: usize, end: usize) -> Vec<(usize, String, &'static str)> {
    let mut out = Vec::new();
    let end = end.min(lines.len());
    let start = start.min(end);
    for (i, line) in lines.iter().enumerate().take(end).skip(start) {
        if is_tool_line(line) {
            continue;
        }
        out.extend(line_tells(i, line));
    }
    out
}

/// Every tell in a `### ` heading's text at absolute line `line`: a loaded
/// opening, "and"/"or" joining two questions, plus the same provenance/
/// code-state/date scan the body gets. Same result shape as
/// `prose_tells_in`, so both feed the same message formatter.
pub fn heading_tells(line: usize, heading_text: &str) -> Vec<(usize, String, &'static str)> {
    let mut out = Vec::new();
    let hw = words(heading_text);
    for form in LOADED_OPENINGS {
        let fw = words(form);
        if hw.len() >= fw.len() && hw[..fw.len()] == fw[..] {
            out.push((line, (*form).to_string(), LOADED_OPENING_WHAT));
        }
    }
    if let Some(w) = hw.iter().find(|w| w.as_str() == "and" || w.as_str() == "or") {
        out.push((line, w.clone(), JOIN_WHAT));
    }
    out.extend(line_tells(line, heading_text));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_hits_and_misses_whole_words_only() {
        assert!(has_phrase(&words("the owner said yes"), "said"));
        assert!(has_phrase(&words("as agreed in the room"), "as agreed"));
        assert!(!has_phrase(&words("this sedative works"), "said"));
    }

    #[test]
    fn code_state_hits_and_misses_whole_words_only() {
        assert!(has_phrase(&words("the tool as is today"), "as is"));
        assert!(!has_phrase(&words("the report has issues"), "as is"));
    }

    #[test]
    fn provenance_list_names_every_agent() {
        for name in ["claude", "codex", "gemini", "copilot"] {
            assert!(PROVENANCE.contains(&name), "{} missing from PROVENANCE", name);
        }
    }

    #[test]
    fn find_date_matches_and_misses() {
        assert_eq!(find_date("decided 2026-09-23 in chat"), Some("2026-09-23"));
        assert_eq!(find_date("no date here"), None);
        assert_eq!(find_date("version 20260923 has no dashes"), None);
    }

    #[test]
    fn prose_tells_in_skips_tool_lines() {
        let lines: Vec<String> = [
            "### does the tool exist?",
            "",
            "id: abc",
            "depends: term",
            "terms: x=1",
            "supersedes: y",
            "**Answer:** A",
            "Problem: claude already built it",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let hits = prose_tells_in(&lines, 0, lines.len());
        // Only the last line is prose; it carries two tells (provenance +
        // code-state).
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|(i, _, _)| *i == 7));
    }

    #[test]
    fn heading_tells_loaded_opening() {
        let hits = heading_tells(0, "which store does the tool use");
        assert!(hits.iter().any(|(_, phrase, what)| phrase == "which" && *what == LOADED_OPENING_WHAT));
    }

    #[test]
    fn heading_tells_join() {
        let hits = heading_tells(0, "does it store terms and definitions");
        assert!(hits.iter().any(|(_, phrase, what)| phrase == "and" && *what == JOIN_WHAT));
    }

    #[test]
    fn heading_tells_clean_heading_has_none() {
        assert!(heading_tells(0, "does the tool store terms").is_empty());
    }

}
