use super::Failure;
use crate::actions;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, rel_path};
use crate::terms;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

/// `terms`: shape/approval failures on `adrdom/terms.md` (read from the
/// index when `staged` and the file is staged there, otherwise the working
/// tree). Missing entirely is a failure once any topic file exists, and a
/// note otherwise. Unapproved or differing from the latest approval is, on
/// its own, only ever a note ("waiting") -- per "Does check fail a terms
/// file that differs from its latest approval?" (03b2853b A) it becomes a
/// failure only when the definition of a term some current decision pairs,
/// directly or through another term's own definition, has actually moved
/// since that approval (`affected_by_edit`). Every asked/answered section's
/// terms line, and every non-retired term's own terms line, is checked for
/// an existing, non-retired uuid on each pair -- never by re-resolving the
/// underlying text. Circular definitions fail. Returns the failures,
/// whether the terms file itself is still awaiting approval, and any notes
/// ("paired by nothing").
pub(super) fn check_terms(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    staged: bool,
) -> (Vec<Failure>, bool, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let has_records = !repo::list_decision_files(root).is_empty();

    let Some(content) = read_terms_content(root, staged) else {
        if has_records {
            out.push(Failure {
                check: "terms".to_string(),
                id: None,
                message: msg::terms_missing(),
            });
        }
        return (out, false, notes);
    };

    let lines = record::split_lines(&content);
    let tf = terms::parse(&lines);

    // A missing id is not a shape error the agent can fix: only approve
    // mints ids. It is reported once, as part of the approval state below,
    // and only fails when a topic file exists -- never as one of
    // `terms_refusals`' shape messages, which run on a minted copy (see
    // below) and so never see it.
    let missing_ids = tf.errors.iter().any(|e| e.contains("missing id"));

    let file_hash = terms::file_hash(&tf);
    let approved = led
        .latest_for_path("terms", "adrdom/terms.md")
        .map(|r| r.hash == file_hash)
        .unwrap_or(false);

    // The approved snapshot (`adrdom/state/terms.approved.md`) against the
    // ledger's own record of what was approved: they are written together,
    // in the same run, by `do_approve_terms`, so a mismatch means the
    // snapshot was edited directly (never an agent's move -- it lives under
    // `adrdom/state/`) or otherwise fell out of sync with the ledger.
    // Checked unconditionally, before anything below trusts the snapshot as
    // the file's latest approved text.
    let snapshot = state::load_terms_snapshot(root);
    if let Some(snap) = &snapshot {
        let snapshot_hash = terms::file_hash(snap);
        let ledger_hash = led.latest_for_path("terms", "adrdom/terms.md").map(|r| r.hash.clone());
        if ledger_hash.as_deref() != Some(snapshot_hash.as_str()) {
            out.push(Failure {
                check: "terms".to_string(),
                id: None,
                message: msg::terms_snapshot_mismatch(),
            });
        }
    }

    // "Does check fail a terms file that differs from its latest
    // approval?" (03b2853b A, supersedes "Does the owner approve the
    // terms file as a whole?"): a draft is committed like a draft plan --
    // never a failure on its own -- except for the one edit that moves
    // what a current decision's answer means. `has_records` no longer
    // matters here: with no current decision, nothing can be paired, so
    // `affected_by_edit` is empty and this always falls to `awaiting`.
    let mut awaiting = false;
    if !approved || missing_ids {
        match affected_by_edit(records, led, &tf, snapshot.as_ref()) {
            EditImpact::DefinitionMoved(terms_list, decisions) => {
                out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: msg::terms_definition_moved(&terms_list, &decisions),
                });
            }
            EditImpact::MissingFromApprovedCopy(terms_list, decisions) => {
                out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: msg::terms_term_missing_from_approved_copy(&terms_list, &decisions),
                });
            }
            EditImpact::SnapshotMissing => {
                out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: msg::terms_snapshot_missing(),
                });
            }
            EditImpact::None => awaiting = true,
        }
    }

    // Shared with approve: every shape, circular-definition,
    // dependent-pairing, and banned-word refusal `adrdom/terms.md` earns,
    // computed on a copy with every id minted in memory so a cycle or a
    // dependent's pairing in a terms file that has never been approved --
    // and so has no ids on disk -- is caught the same way `approve` catches
    // it (`terms::cycles` and `terms::phrase_lookup` only see a term that
    // has an id).
    let minted = terms::minted_copy(&tf);
    let (refusals, _pairs) = actions::terms_refusals(root, records, &minted, led);
    for r in refusals {
        out.push(Failure {
            check: "terms".to_string(),
            id: None,
            message: r,
        });
    }

    // Every asked/answered section: must have a terms line; each uuid on
    // it must name a term that exists and is not retired. No
    // re-resolution of the section text against today's terms.
    for rf in records {
        let rel = rel_path(root, &rf.path);
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            let st = led.latest_state(id).map(|r| r.event.as_str());
            if !matches!(st, Some("asked") | Some("answered")) {
                continue;
            }
            let Some(tl) = sec.terms_line else {
                out.push(Failure {
                    check: "terms".to_string(),
                    id: Some(id.clone()),
                    message: msg::section_no_terms_line(&rel, &sec.heading_text),
                });
                continue;
            };
            let pairs = terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
            for (_, uuid) in &pairs {
                match tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
                    None => out.push(Failure {
                        check: "terms".to_string(),
                        id: Some(id.clone()),
                        message: msg::section_term_not_found(&rel, &sec.heading_text, uuid),
                    }),
                    Some(t) if t.retired => out.push(Failure {
                        check: "terms".to_string(),
                        id: Some(id.clone()),
                        message: msg::section_term_retired(&rel, &sec.heading_text, &t.name),
                    }),
                    Some(_) => {}
                }
            }
        }
    }

    // Every non-retired term's own terms line: the same two checks.
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(tl) = &t.terms_line else { continue };
        let pairs = terms::parse_terms_line(tl).unwrap_or_default();
        for (_, uuid) in &pairs {
            match tf.terms.iter().find(|o| o.id.as_deref() == Some(uuid.as_str())) {
                None => out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: msg::term_definition_term_not_found(&t.name, uuid),
                }),
                Some(o) if o.retired => out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: msg::term_definition_term_retired(&t.name, &o.name),
                }),
                Some(_) => {}
            }
        }
    }

    // Note (never a failure): every non-retired term that no current
    // decision's terms line, and no non-retired term's own terms line,
    // pairs against.
    let mut used: HashSet<String> = HashSet::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            if let Some(pairs) = terms::parse_terms_line(rf.lines[tl].trim_end()) {
                for (_, uuid) in pairs {
                    used.insert(uuid);
                }
            }
        }
    }
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(tl) = &t.terms_line else { continue };
        if let Some(pairs) = terms::parse_terms_line(tl) {
            for (_, uuid) in pairs {
                used.insert(uuid);
            }
        }
    }
    let mut unused: Vec<String> = tf
        .terms
        .iter()
        .filter(|t| !t.retired)
        .filter(|t| t.id.as_ref().map(|id| !used.contains(id)).unwrap_or(true))
        .map(|t| t.name.clone())
        .collect();
    unused.sort();
    if !unused.is_empty() {
        notes.push(msg::terms_unpaired(unused.len(), &unused.join(", ")));
    }

    (out, awaiting, notes)
}

/// Every term id that some current (`asked`/`answered`, still `is_current`)
/// decision's terms line pairs, together with the current decisions
/// (`<rel>: "<heading>"`) that reach it -- directly, or transitively
/// through another term's own terms line, since that term's definition is
/// where the pairing word actually lives (per "Does check fail a terms
/// file that differs from its latest approval?": "directly, or through
/// another term's definition"). Mirrors `check_terms`' own "paired by
/// nothing" note above, which likewise reads only `sec.terms_line` and a
/// term's own `terms_line` as "pairing" -- never a bare `depends:` entry.
///
/// `tf` supplies the graph edges (a term's own `terms_line`, which only
/// `approve` ever writes, so it reads the same whether `tf` is a clean,
/// already-approved file or a draft with an unapproved edit sitting on top
/// of it -- an agent never touches that line itself).
fn reachable_terms(
    records: &[RecordFile],
    led: &Ledger,
    tf: &terms::TermsFile,
) -> HashMap<String, BTreeSet<String>> {
    let mut reach: HashMap<String, BTreeSet<String>> = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            let Some(pairs) = terms::parse_terms_line(rf.lines[tl].trim_end()) else {
                continue;
            };
            let source = format!("{}: \"{}\"", rf.rel, sec.heading_text);
            let mut seen: HashSet<String> = HashSet::new();
            let mut stack: Vec<String> = pairs.into_iter().map(|(_, uuid)| uuid).collect();
            while let Some(uuid) = stack.pop() {
                if !seen.insert(uuid.clone()) {
                    continue;
                }
                reach.entry(uuid.clone()).or_default().insert(source.clone());
                let Some(t) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
                else {
                    continue;
                };
                if let Some(more) = t.terms_line.as_deref().and_then(terms::parse_terms_line) {
                    stack.extend(more.into_iter().map(|(_, u)| u));
                }
            }
        }
    }
    reach
}

/// The outcome of comparing every term a current decision reaches
/// (`reachable_terms`) against the terms file's latest approved text (per
/// "Does check fail a terms file that differs from its latest approval?").
enum EditImpact {
    /// Nothing reached has moved (including the case where nothing is
    /// reached at all): the caller reports the file as merely waiting.
    None,
    /// At least one reached term's definition differs from the approved
    /// snapshot: `(term names, decisions)`, both already joined for
    /// printing.
    DefinitionMoved(String, String),
    /// At least one reached term is present in the current draft but is
    /// missing from the approved snapshot entirely -- its own case, not
    /// folded into `DefinitionMoved` (which needs the term in both to
    /// compare a "before" against an "after"): `(term names, decisions)`,
    /// both already joined for printing.
    MissingFromApprovedCopy(String, String),
    /// Something is reached, but there is no approved snapshot to compare
    /// against at all (the file has never been approved since this
    /// mechanism existed, or the snapshot is unreadable): fails safe rather
    /// than guessing, per 03b2853b A's own cost line ("a typo in a paired
    /// definition still waits for the owner") -- a paired term waits on
    /// anything that might be a change, not only a proven one.
    SnapshotMissing,
}

/// The failure case of "Does check fail a terms file that differs from its
/// latest approval?" (03b2853b A): compares every term a current decision
/// reaches (`reachable_terms`) against its text in `old_tf` -- the terms
/// file's latest approved snapshot (`state::load_terms_snapshot`), read by
/// the caller once and passed in here rather than reloaded per call.
fn affected_by_edit(
    records: &[RecordFile],
    led: &Ledger,
    tf: &terms::TermsFile,
    old_tf: Option<&terms::TermsFile>,
) -> EditImpact {
    let reach = reachable_terms(records, led, tf);
    if reach.is_empty() {
        return EditImpact::None;
    }
    let Some(old_tf) = old_tf else {
        return EditImpact::SnapshotMissing;
    };

    let mut changed_names: BTreeSet<String> = BTreeSet::new();
    let mut decisions: BTreeSet<String> = BTreeSet::new();
    let mut missing_names: BTreeSet<String> = BTreeSet::new();
    let mut missing_decisions: BTreeSet<String> = BTreeSet::new();
    for (uuid, sources) in &reach {
        let Some(cur) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) else {
            continue;
        };
        match old_tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
            // Present in both: a real "before" to compare the "after"
            // against.
            Some(o) => {
                if o.definition != cur.definition {
                    changed_names.insert(cur.name.clone());
                    decisions.extend(sources.iter().cloned());
                }
            }
            // Present now but not in the approved snapshot at all: its own
            // case, reported separately below rather than folded into
            // "changed" (there is no "before" text here to say it moved
            // from -- `unwrap_or(true)` used to treat this the same as a
            // real change).
            None => {
                missing_names.insert(cur.name.clone());
                missing_decisions.extend(sources.iter().cloned());
            }
        }
    }

    if !missing_names.is_empty() {
        return EditImpact::MissingFromApprovedCopy(
            missing_names.into_iter().collect::<Vec<_>>().join(", "),
            missing_decisions.into_iter().collect::<Vec<_>>().join(", "),
        );
    }
    if changed_names.is_empty() {
        return EditImpact::None;
    }
    EditImpact::DefinitionMoved(
        changed_names.into_iter().collect::<Vec<_>>().join(", "),
        decisions.into_iter().collect::<Vec<_>>().join(", "),
    )
}

/// `adrdom/terms.md`'s content for `check`: when `staged`, the index copy
/// whenever the file is tracked in the index at all (whether or not this
/// commit changes it), so a dirty unstaged working-tree edit never leaks
/// into a staged check; the working tree is used only when the path isn't
/// in the index. When not `staged`, always the working tree. `None` when
/// the file exists in neither place.
fn read_terms_content(root: &Path, staged: bool) -> Option<String> {
    if staged {
        let tracked = repo::git(
            root,
            &["ls-files", "--error-unmatch", "adrdom/terms.md"],
        )
        .is_some();
        if tracked {
            // In the index: the index copy is the answer either way -- a
            // read failure here is not a reason to fall back to the
            // (untrusted, possibly dirty) working tree.
            return repo::git(root, &["show", ":adrdom/terms.md"]);
        }
    }
    std::fs::read_to_string(root.join("adrdom/terms.md")).ok()
}
