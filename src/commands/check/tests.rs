use crate::rules::Class;
use super::Failure;
use crate::ledger::Ledger;
use crate::rules::{Finding, Subject};
use crate::msg;
use crate::record::RecordFile;
use crate::repo::{self, Source};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Every id that appears after `adr: ` in a tracked, non-`adrdom/` file, as
/// a full uuid token, mapped to every `path:line` location it was found at
/// (what `check`'s untested findings and the build moves read). Which files are
/// tracked always comes from `git ls-files` (the index), in both `source`s
/// alike -- a tracked-only scan, per "What ties a test to a decision?"
/// ("every tracked file outside adrdom"), so this never walks a
/// gitignored build directory; only each file's *content* is read through
/// `source`, so a staged edit to a tie is judged, not whatever is left on
/// disk (per "Does check at a commit read the staged files?").
pub fn test_index(root: &Path, source: &Source) -> HashMap<String, Vec<String>> {
    let mut test_ids: HashMap<String, Vec<String>> = HashMap::new();
    let adrdom_prefix = format!("{}/", repo::ADRDOM_DIR);
    if let Some(list) = repo::git_paths(root, &["ls-files"]) {
        for path in &list {
            if path.starts_with(&adrdom_prefix) {
                continue;
            }
            let Some(content) = source.read(root, path) else {
                continue;
            };
            for (i, line) in content.lines().enumerate() {
                if let Some(pos) = line.find("adr: ") {
                    let rest = &line[pos + 5..];
                    let id_tok: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                    if crate::record::is_uuid(&id_tok) {
                        test_ids
                            .entry(id_tok)
                            .or_default()
                            .push(msg::location(path, i + 1));
                    }
                }
            }
        }
    }
    test_ids
}

pub(super) fn check_tests(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    tests: Option<&HashMap<String, Vec<String>>>,
    source: &Source,
) -> (Vec<Failure>, Vec<Finding>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();

    let current: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .filter(|id| matches!(led.latest_state(id), Some(row) if row.event == "answered"))
        .collect();

    let owned;
    let test_ids: &HashMap<String, Vec<String>> = match tests {
        Some(t) => t,
        None => {
            owned = test_index(root, source);
            &owned
        }
    };

    // Where each question sits: a line that names an identifier names the
    // question beside it, whatever its state.
    let mut place: HashMap<&str, (&RecordFile, usize)> = HashMap::new();
    for rf in records {
        for (i, s) in rf.sections.iter().enumerate() {
            if let Some(id) = &s.id {
                place.insert(id.as_str(), (rf, i));
            }
        }
    }

    // In the order of the identifiers, so two runs print the same lines.
    let mut untested: Vec<&String> = current.iter().filter(|id| !test_ids.contains_key(*id)).collect();
    untested.sort();
    for id in untested {
        // Per "What does check do with an untested current decision?": a
        // note on every run, never a failure, with the move that builds it
        // (per "Does check print a line that names no move?", 3fc35d4f A).
        let Some(&(rf, i)) = place.get(id.as_str()) else { continue };
        // Two standing answers: nothing stands to build; the conflict's own
        // finding names the move, as `show` names it.
        if crate::state::conflicting_rulings(led, id).is_some() {
            continue;
        }
        // A file whose tiers do not derive has no tier to wait on yet: its
        // own finding names the move, and `show` names no build for it;
        // the untested line says so.
        let mv = if rf.tiers.is_none() {
            crate::moves::Move::new(crate::moves::MoveKind::BuildWaitsTiers { heading: rf.sections[i].heading_text.clone() })
        } else {
            crate::moves::of_untested(rf, i, id)
        };
        notes.push(
            Finding::said(Class::Untested, msg::untested(&rf.rel, &rf.sections[i].heading_text, &mv.text()))
                .about(Subject::TopicFile(rf.rel.clone()))
                .in_section(i),
        );
    }

    let mut stale: Vec<(&String, &Vec<String>)> =
        test_ids.iter().filter(|(id, _)| !current.contains(*id)).collect();
    stale.sort();
    for (id, locations) in stale {
        let heading = place.get(id.as_str()).map(|&(rf, i)| rf.sections[i].heading_text.as_str());
        out.push(Failure::said(Class::Tested, msg::stale_test(id, heading, locations)));
    }

    (out, notes)
}

