//! Conformance of this repository's own records to the current decisions
//! about record text. These tests read the real files under
//! `adrdom/decisions/`, not a fixture.

use std::fs;
use std::path::PathBuf;

fn records_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("adrdom/decisions")
}

/// Every line of every topic file, with its path and 1-based line number.
fn record_lines() -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(records_dir()) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap();
        for (i, line) in content.lines().enumerate() {
            out.push((path.display().to_string(), i + 1, line.to_string()));
        }
    }
    out
}

/// A line the tool writes or that only points at other text: never the
/// agent's own prose, so never scanned for tells.
fn is_tool_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("id: ")
        || t.starts_with("terms: ")
        || t.starts_with("depends: ")
        || t.starts_with("**Answer:**")
}

/// The text of every `### ` question heading, without the marker.
fn heading_lines() -> Vec<(String, usize, String)> {
    record_lines()
        .into_iter()
        .filter_map(|(path, no, line)| {
            line.strip_prefix("### ")
                .map(|h| (path, no, h.trim().to_string()))
        })
        .collect()
}

/// Lowercased runs of alphanumerics: the words of a line, so that a tell
/// matches whole words only ("as is" never matches inside "has issues").
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whether `tell`'s words occur contiguously in `line_words`.
fn has_phrase(line_words: &[String], tell: &str) -> bool {
    let tw = words(tell);
    !tw.is_empty() && line_words.windows(tw.len()).any(|w| w == tw.as_slice())
}

/// A date of the form YYYY-MM-DD anywhere in the line.
fn has_date(line: &str) -> bool {
    let b = line.as_bytes();
    (0..b.len().saturating_sub(9)).any(|i| {
        b[i..i + 4].iter().all(u8::is_ascii_digit)
            && b[i + 4] == b'-'
            && b[i + 5..i + 7].iter().all(u8::is_ascii_digit)
            && b[i + 7] == b'-'
            && b[i + 8..i + 10].iter().all(u8::is_ascii_digit)
    })
}

#[test]
fn record_text_cites_neither_provenance_nor_code_state() {
    // Who said, chose, or wanted what; where and when it was said. No actor
    // word: the phrases must hold in any repository, whatever it calls its
    // people and its agents.
    let provenance = [
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
    ];
    // What the code does at the moment of writing, or an option that only
    // ratifies it.
    let code_state = [
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
    let mut hits = Vec::new();
    for (path, line_no, line) in record_lines() {
        if is_tool_line(&line) {
            continue;
        }
        let lw = words(&line);
        for tell in provenance.iter().chain(code_state.iter()) {
            if has_phrase(&lw, tell) {
                hits.push(format!("{}:{}: \"{}\"", path, line_no, tell));
            }
        }
        if has_date(&line) {
            hits.push(format!("{}:{}: a date", path, line_no));
        }
    }
    assert!(hits.is_empty(), "record lines citing provenance or code state:\n{}", hits.join("\n"));
}

#[test]
fn heading_does_not_open_with_a_loaded_form() {
    // A heading that opens with one of these presupposes the set, the
    // place, or the count it asks about; the fork the owner has not
    // decided is whether the thing exists at all.
    let loaded_forms = ["which", "where", "how many", "how much"];
    let mut hits = Vec::new();
    for (path, line_no, heading) in heading_lines() {
        let hw = words(&heading);
        for form in loaded_forms.iter() {
            let fw = words(form);
            if hw.len() >= fw.len() && hw[..fw.len()] == fw[..] {
                hits.push(format!("{}:{}: \"{}\"", path, line_no, heading));
            }
        }
    }
    assert!(hits.is_empty(), "headings opening with a loaded form:\n{}", hits.join("\n"));
}

#[test]
fn heading_asks_one_thing() {
    // "and" or "or" in a heading joins two forks into one question; each
    // gets its own heading.
    let mut hits = Vec::new();
    for (path, line_no, heading) in heading_lines() {
        let hw = words(&heading);
        if hw.iter().any(|w| w == "and" || w == "or") {
            hits.push(format!("{}:{}: \"{}\"", path, line_no, heading));
        }
    }
    assert!(hits.is_empty(), "headings joining more than one question:\n{}", hits.join("\n"));
}
