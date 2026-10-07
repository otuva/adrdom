// Every status a command prints is a name from the closed lists of
// src/status.rs, and no other source file words a status of its own (per
// "Does a command name a status of its own?", f53ac3b9 A).
mod common;

use adr::status::{FileStatus, QuestionStatus, TermStatus};
use common::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const REL: &str = "adrdom/decisions/gate.md";

// adr: f53ac3b9-efde-427c-8027-19988aa8fda7
//
// Each closed list names each of its statuses once, and its `ALL` holds
// every variant: a match on the variants with no wildcard arm fails to
// compile the day a status is added and `ALL` is not.
#[test]
fn every_closed_list_is_whole_and_names_each_status_once() {
    for s in QuestionStatus::ALL {
        match s {
            QuestionStatus::NoBody
            | QuestionStatus::NotAsked
            | QuestionStatus::Asked
            | QuestionStatus::ChangedSinceAsked
            | QuestionStatus::Answered
            | QuestionStatus::Refused
            | QuestionStatus::Superseded
            | QuestionStatus::Withdrawn
            | QuestionStatus::Dropped => {}
        }
    }
    for s in FileStatus::ALL {
        match s {
            FileStatus::New | FileStatus::Changed | FileStatus::Approved | FileStatus::Removed => {}
        }
    }
    for s in TermStatus::ALL {
        match s {
            TermStatus::New | TermStatus::Changed | TermStatus::Approved | TermStatus::Retired | TermStatus::Removed => {}
        }
    }
    assert_eq!(QuestionStatus::ALL.len(), 9);
    assert_eq!(FileStatus::ALL.len(), 4);
    assert_eq!(TermStatus::ALL.len(), 5);
    let q: HashSet<&str> = QuestionStatus::ALL.iter().map(|s| s.name()).collect();
    let f: HashSet<&str> = FileStatus::ALL.iter().map(|s| s.name()).collect();
    let t: HashSet<&str> = TermStatus::ALL.iter().map(|s| s.name()).collect();
    assert_eq!(q.len(), QuestionStatus::ALL.len(), "two question statuses share a name");
    assert_eq!(f.len(), FileStatus::ALL.len(), "two file statuses share a name");
    assert_eq!(t.len(), TermStatus::ALL.len(), "two term statuses share a name");
}

/// Every name a status may be printed by: the names of the closed lists.
fn names() -> HashSet<&'static str> {
    QuestionStatus::ALL
        .iter()
        .map(|s| s.name())
        .chain(FileStatus::ALL.iter().map(|s| s.name()))
        .chain(TermStatus::ALL.iter().map(|s| s.name()))
        .collect()
}

/// The status of each line `show` opens with one, `[<status>]` (after the
/// `+ ` or `- ` of a thing new or struck as a whole): the name
/// with an answered question's letter or an asked one's version taken off.
fn bracketed(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.strip_prefix("+ ").or_else(|| l.strip_prefix("- ")).unwrap_or(l))
        .filter_map(|l| l.strip_prefix('['))
        .filter_map(|l| l.split_once(']').map(|(s, _)| s.to_string()))
        .map(|s| match s.split_once(" @") {
            Some((name, _)) => name.to_string(),
            None => s,
        })
        .map(|s| match s.strip_prefix("answered ") {
            Some(letter) if letter.len() == 1 => "answered".to_string(),
            _ => s,
        })
        .collect()
}

fn show(dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["show"];
    all.extend_from_slice(args);
    let out = adr(dir, &all, &[], "");
    stdout(&out) + &stderr(&out)
}

fn id_of(path: &Path, heading: &str) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let at = text.find(&format!("### {}\n", heading)).unwrap();
    extract_ids(&text[at..])[0].clone()
}

fn commit_all(dir: &Path) {
    run_git(dir, &["add", "-A"]);
    assert!(run_git(dir, &["commit", "-q", "-m", "step"]).status.success());
}

// adr: f53ac3b9-efde-427c-8027-19988aa8fda7
//
// A topic file walked through every status a question takes, and the terms
// file and the topic file through theirs: every bracketed status `show`
// prints is a name of a closed list, and every name of the question list
// is printed at some step.
#[test]
fn show_prints_every_status_by_its_name() {
    let dir = widget_repo();
    let heads = ["Does the widget hold?", "Does the widget bend?", "Does the widget fold?", "Does the widget turn?", "Does the widget spin?"];
    // The fifth stands on the first, so no tier holds more than the cap.
    let questions: String = heads
        .iter()
        .map(|h| gate_question(h, "the widget breaks."))
        .map(|q| q.replacen("depends: widget\n", &format!("depends: widget, {}=*\n", heads[0]), usize::from(q.starts_with(&format!("### {}", heads[4])))))
        .collect::<Vec<_>>()
        .join("\n");
    let mut seen: Vec<String> = Vec::new();

    // New, with a heading that has no body yet.
    let path: PathBuf = write_record(&dir, REL, &format!("# Gate\n\n{}\n### Does the widget stay?\ndepends: widget\n", questions));
    seen.extend(bracketed(&show(&dir, &[REL])));
    seen.extend(bracketed(&show(&dir, &["adrdom/terms.md"])));

    // Approved, then asked.
    write_record(&dir, REL, &format!("# Gate\n\n{}", questions));
    approve_record(&dir, REL);
    commit_all(&dir);
    seen.extend(bracketed(&show(&dir, &[REL])));
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    commit_all(&dir);
    seen.extend(bracketed(&show(&dir, &[REL])));

    // Asked, then its text edited: changed since asked; put back as asked.
    let asked = std::fs::read_to_string(&path).unwrap();
    let at = asked.find(&format!("### {}\n", heads[0])).unwrap();
    let edited = format!("{}{}", &asked[..at], asked[at..].replacen("Problem: ", "Problem: now ", 1));
    std::fs::write(&path, &edited).unwrap();
    let shown = show(&dir, &[REL]);
    assert!(shown.contains(&format!("[changed since asked] {}", heads[0])), "{}", shown);
    seen.extend(bracketed(&shown));
    std::fs::write(&path, &asked).unwrap();

    // Answered and refused by the owner; superseded, withdrawn and dropped
    // as the ledger records them.
    assert!(adr_answer(&dir, &id_of(&path, heads[0]), "A").status.success());
    assert!(adr_answer(&dir, &id_of(&path, heads[1]), "refuse").status.success());
    for (event, head) in [("superseded", heads[2]), ("withdrawn", heads[3]), ("dropped", heads[4])] {
        append_ledger_row_at(&dir, event, &id_of(&path, head), "h", REL, "2999-01-01T00:00:00Z");
    }
    seen.extend(bracketed(&show(&dir, &[REL])));

    // Changed: the prose edited after the approval.
    let text = std::fs::read_to_string(&path).unwrap().replacen("# Gate\n", "# Gate\n\nThe widget gate.\n", 1);
    std::fs::write(&path, text).unwrap();
    seen.extend(bracketed(&show(&dir, &[REL])));
    seen.extend(bracketed(&show(&dir, &["--all"])));

    let names = names();
    for s in &seen {
        assert!(names.contains(s.as_str()), "a status printed by no name of status.rs: [{}]", s);
    }
    for q in QuestionStatus::ALL {
        assert!(seen.iter().any(|s| s == q.name()), "[{}] was never printed: {:?}", q.name(), seen);
    }
    for f in [FileStatus::New, FileStatus::Approved, FileStatus::Changed] {
        assert!(seen.iter().any(|s| s == f.name()), "[{}] was never printed: {:?}", f.name(), seen);
    }
}

// adr: f53ac3b9-efde-427c-8027-19988aa8fda7
//
// No source file but status.rs words a status itself: the words a status
// was once spelled with in the messages (a file "awaiting approval", "not
// approved", "new or changed"; a question with "no body" or "not asked")
// appear in a string of no other file.
#[test]
fn no_source_file_but_status_words_a_status() {
    let phrases = [
        "awaiting approval",
        "not approved",
        "new or changed",
        "waits for approval",
        "waiting for approval",
        "not yet approved",
        "no body",
        "not asked",
    ];
    let mut found = Vec::new();
    for (rel, text) in source_files() {
        if rel == "src/status.rs" {
            continue;
        }
        for (n, line) in code_lines(&text) {
            for lit in string_literals(line) {
                let lower = lit.to_lowercase();
                if phrases.iter().any(|ph| lower.contains(ph)) {
                    found.push(format!("{}:{}: \"{}\"", rel, n, lit));
                }
            }
        }
    }
    found.sort();
    assert!(found.is_empty(), "status words outside src/status.rs:\n{}", found.join("\n"));
}
