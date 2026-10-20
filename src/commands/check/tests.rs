use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::repo::{self, Source};
use crate::rules::Class;
use crate::rules::{Finding, Subject};
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
    scan(root, source, "", &mut test_ids);
    // Per "Does check in the primary count a tie in a satellite?": the primary
    // also reads each satellite of its list that has a checkout, from its
    // working tree. One that is missing is not scanned, never cloned.
    for row in crate::satellite::rows(root) {
        let dir = crate::satellite::row_dir(root, &row);
        if !dir.join(".git").exists() {
            continue;
        }
        scan(
            &dir,
            &Source::WorkingTree,
            &format!("{}/", row.path),
            &mut test_ids,
        );
    }
    test_ids
}

/// The note for a satellite with no checkout at its path.
pub(crate) fn satellite_missing_finding(row: &crate::satellite::Row) -> Finding {
    Finding::said(Class::Untested, msg::satellite_missing(&row.path, &row.url))
}

/// Adds every tie of the tracked files of the repository at `root` to
/// `test_ids`, each location led by `prefix`.
fn scan(root: &Path, source: &Source, prefix: &str, test_ids: &mut HashMap<String, Vec<String>>) {
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
                            .push(msg::location(&format!("{}{}", prefix, path), i + 1));
                    }
                }
            }
        }
    }
}

pub(super) fn check_tests(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    tests: Option<&HashMap<String, Vec<String>>>,
    source: &Source,
    role: crate::session::Role,
    in_satellite: bool,
) -> Vec<Finding> {
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

    // Per "Does check in a satellite report an untested decision?" (No), and
    // per "What does check in the primary report for a satellite missing at
    // its path?": the satellite is named with its url, and no decision is
    // untested until it is found.
    let missing = if in_satellite {
        Vec::new()
    } else {
        crate::satellite::missing(root)
    };
    for row in &missing {
        notes.push(satellite_missing_finding(row));
    }

    // In the order of the identifiers, so two runs print the same lines.
    let builds = crate::satellite::builds(in_satellite, &missing);
    let mut untested: Vec<&String> = current
        .iter()
        .filter(|id| builds && !test_ids.contains_key(*id))
        .collect();
    untested.sort();
    for id in untested {
        // Per "What does check do with an untested current decision?": a
        // note on every run, never a failure, with the move that builds it
        // (per "Does check print a line with no move on it?", bd83aa22 A).
        let Some(&(rf, i)) = place.get(id.as_str()) else {
            continue;
        };
        // Two standing answers: nothing stands to build; the conflict's own
        // finding names the move, as `show` names it.
        if crate::state::conflicting_rulings(led, id).is_some() {
            continue;
        }
        // A file whose tiers do not derive has no tier to wait on yet: its
        // own finding names the move, and `show` names no build for it;
        // the untested line says so.
        let mv = if rf.tiers.is_none() {
            crate::moves::Move::new(crate::moves::MoveKind::BuildWaitsTiers {
                heading: rf.sections[i].heading_text.clone(),
            })
        } else {
            crate::moves::of_untested(rf, i, id, role)
        };
        notes.push(
            Finding::said(
                Class::Untested,
                msg::untested(&rf.rel, &rf.sections[i].heading_text, mv.clause()),
            )
            .about(Subject::TopicFile(rf.rel.clone()))
            .in_section(i),
        );
    }

    let mut stale: Vec<(&String, &Vec<String>)> = test_ids
        .iter()
        .filter(|(id, _)| !current.contains(*id))
        .collect();
    stale.sort();
    for (id, locations) in stale {
        // A superseded question no file holds is named by the heading its
        // `asked` row carries (adr-code: b5ca161f-c88c-4f02-a9d9-1d406cf3a252).
        let heading = place
            .get(id.as_str())
            .map(|&(rf, i)| rf.sections[i].heading_text.as_str())
            .or_else(|| {
                led.latest_asked(id)
                    .map(|r| r.detail.as_str())
                    .filter(|d| !d.is_empty())
            });
        // The tie moves to the current decisions that took the decision's
        // place, or goes when none did (2381d759): a note, never a failure.
        let to: Vec<(String, String)> = crate::state::current_successors(led, id)
            .into_iter()
            .filter_map(|n| {
                place
                    .get(n.as_str())
                    .map(|&(rf, i)| (rf.sections[i].heading_text.clone(), n.clone()))
            })
            .collect();
        let mv = crate::moves::Move::new(crate::moves::MoveKind::Retie { to }).in_session(role);
        notes.push(Finding::said(
            Class::Tested,
            msg::stale_test(id, heading, locations, mv.clause()),
        ));
    }

    notes
}
