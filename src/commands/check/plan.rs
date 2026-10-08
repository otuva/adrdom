use crate::rules::Class;
use super::{is_note, Failure};
use crate::rules::{self, Finding};
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::repo::{self, Source};
use crate::state;
use std::collections::HashMap;
use std::path::Path;

/// `plan`: the plan findings `approve` refuses on (per "What does a depends
/// line name?", "Does the tool derive tiers from the depends lines?", "Does
/// the tool cap the questions in a tier?", "Does a depends entry carry the
/// answers it holds under?", "Does the agent write the bodies before
/// approve?" and the planning records of "What does the agent do to plan a
/// topic file?" -- the `plan` findings of the file's judgement, computed
/// once in `mod.rs` and shared with `approve`'s `do_approve_record`).
/// On a file whose plan and prose are approved each is a failure; on a file
/// awaiting approval each is a note instead (per "Does check fail a topic
/// file awaiting approval?": every plan finding on such a file behaves the
/// same way, never a failure). A frozen section's edited depends line stays
/// a failure either way (its own category, `plan_frozen`). Also reports a bad file name and a heading
/// duplicated within one file or across two (23f21671 A): extras read off
/// the whole directory, so none of them is a `record_refusals` category
/// (`approve` refuses the same lines through `rules`). Returns the failures
/// and the notes (the findings above on a file in `awaiting`).
pub(super) fn check_plan(
    root: &Path,
    source: &Source,
    records: &[RecordFile],
    led: &Ledger,
    refusals: &[Vec<Finding>],
    awaiting: &[String],
) -> (Vec<Failure>, Vec<Failure>) {
    let mut out = Vec::new();
    let mut notes: Vec<Failure> = Vec::new();
    let mut heading_files: HashMap<String, Vec<String>> = HashMap::new();
    let records_by_rel: HashMap<&str, &RecordFile> =
        records.iter().map(|rf| (rf.rel.as_str(), rf)).collect();

    // The file name rule (lowercase words joined by hyphens, named by its
    // topic; per "What names a topic file?"): every file `repo::list_decision_files_via` lists through
    // `source`, by its own base name alone -- checked straight off the
    // listing rather than off `records`, so a file whose name is wrong is
    // still told so even when its content also fails to parse into one.
    for f in repo::list_decision_files_via(root, source) {
        let rel = state::rel_path(root, &f);
        let topic = records_by_rel.get(rel.as_str()).and_then(|rf| rf.topic.as_deref());
        // A file awaiting approval earns a note, an approved one a failure
        // (per "Does check fail a topic file awaiting approval?"); a file
        // that did not parse has no approval state read, so it fails.
        let draft = records_by_rel.contains_key(rel.as_str()) && awaiting.contains(&rel);
        for m in rules::own_name_refusals(&rel, topic) {
            push_plan(&mut out, &mut notes, draft, m);
        }
    }

    for (rf, refs) in records.iter().zip(refusals) {
        let rel = rf.rel.clone();
        // The missing-depends-line, entry-resolution, derive-error, tier-cap,
        // letter, body and planning-record findings: the `plan` findings
        // of the file's judgement, computed once in `mod.rs`.
        let draft = awaiting.contains(&rel);
        for r in refs {
            if r.class != Class::Plan && r.class != Class::PlanFrozen {
                continue;
            }
            push_finding(&mut out, &mut notes, is_note(rf, r, draft, led), r.clone());
        }

        // Same heading twice in one file (8.3-adjacent): the cross-file
        // dedup below only reports a heading once per distinct path, so two
        // identical `### ` headings inside this one file would otherwise
        // never surface (and `derive_tiers`' heading index binds a depends
        // line to the first of the two only, silently).
        for sec in &rf.sections {
            heading_files
                .entry(sec.heading_text.clone())
                .or_default()
                .push(rel.clone());
        }
        // The fault belongs to the copying question (`rules::copying_sections`):
        // a note when one sits in a draft, a failure otherwise.
        let mut dup_headings: Vec<&str> = rf.sections.iter().map(|s| s.heading_text.as_str()).collect();
        dup_headings.sort();
        dup_headings.dedup();
        for h in dup_headings {
            let bearing: Vec<_> =
                rf.sections.iter().filter(|s| s.heading_text == h).cloned().collect();
            if bearing.len() < 2 {
                continue;
            }
            let copied = !rules::copying_sections(rf, h, led).is_empty();
            for m in rules::duplicate_heading_refusals(&rel, &bearing) {
                push_plan(&mut out, &mut notes, draft && copied, m);
            }
        }
    }

    // One heading in two topic files (23f21671 A): the fault belongs to the
    // copying question (`rules::copying_sections`, 05141f49 A). A note when
    // one sits in a file awaiting approval, a failure otherwise. The files
    // are compared section by section over every section each one holds,
    // superseded and answered ones included, the set `approve` compares too.
    let mut headings: Vec<&String> = heading_files.keys().collect();
    headings.sort();
    for heading in headings {
        let mut paths = heading_files[heading].clone();
        paths.sort();
        paths.dedup();
        if paths.len() > 1 {
            let copied_in_draft = paths.iter().any(|rel| {
                records_by_rel.get(rel.as_str()).is_some_and(|rf| {
                    awaiting.contains(rel) && !rules::copying_sections(rf, heading, led).is_empty()
                })
            });
            let message = msg::duplicate_heading_across_files(heading, &paths[0], &paths[1]);
            push_plan(&mut out, &mut notes, copied_in_draft, Failure::said(Class::Plan, message));
        }
    }

    (out, notes)
}

/// One plan finding: a note when `note`, a failure otherwise.
fn push_plan(out: &mut Vec<Failure>, notes: &mut Vec<Failure>, note: bool, f: Failure) {
    push_finding(out, notes, note, f.class(Class::Plan));
}

/// One finding: a note when `note`, a failure otherwise.
fn push_finding(out: &mut Vec<Failure>, notes: &mut Vec<Failure>, note: bool, f: Failure) {
    if note {
        notes.push(f);
    } else {
        out.push(f);
    }
}
