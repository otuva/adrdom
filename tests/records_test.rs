//! Conformance of this repository's own records to the decisions in force
//! about record text. These tests read the real files under
//! `adrdom/decisions/`, not a fixture.

use std::fs;
use std::path::PathBuf;

fn records_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("adrdom/decisions")
}

fn rationale_lines() -> Vec<(String, usize, String)> {
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
            let t = line.trim_start();
            if t.starts_with("Why: ") || t.starts_with("Costs: ") {
                out.push((path.display().to_string(), i + 1, t.to_string()));
            }
        }
    }
    out
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
fn rationale_cites_neither_provenance_nor_code_state() {
    // Who asked, chose, or said what; where it was said; when.
    let provenance = [
        "the owner chose",
        "the owner said",
        "the owner asked",
        "the owner decided",
        "the user ",
        "agreed in chat",
        "said in chat",
        "asked in chat",
        "decided in chat",
        "this session",
        "claude",
        "as agreed",
        "as discussed",
    ];
    // What the code does at the moment of writing.
    let code_state = [
        "already does",
        "already has",
        "already writes",
        "already refuses",
        "currently",
        "as built",
        "as implemented",
        "the code already",
        "the tool already",
        "right now",
    ];
    let mut hits = Vec::new();
    for (path, line_no, line) in rationale_lines() {
        let lower = line.to_lowercase();
        for tell in provenance.iter().chain(code_state.iter()) {
            if lower.contains(tell) {
                hits.push(format!("{}:{}: \"{}\"", path, line_no, tell));
            }
        }
        if has_date(&line) {
            hits.push(format!("{}:{}: a date", path, line_no));
        }
    }
    assert!(hits.is_empty(), "rationale lines citing provenance or code state:\n{}", hits.join("\n"));
}
