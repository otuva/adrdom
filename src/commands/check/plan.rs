use super::{file_approval, Failure};
use crate::actions::{self, Refusal};
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::state;
use std::collections::HashMap;
use std::path::Path;

/// `plan`: per file, compares the current plan/context hash against the
/// latest `approved`/`context` ledger rows for that path. Also reports the
/// same depends-line/derivation/cap problems `approve` refuses on (per
/// "What does a depends line name?", "Does the tool derive tiers from the
/// depends lines?", and "Does the tool cap the questions in a tier?" --
/// the `plan` category of `record_refusals`, computed once in
/// `mod.rs` and shared with `approve`'s `do_approve_record`):
/// unconditionally, regardless of the file's approval state (a derive
/// error, a missing depends line, or an entry that resolves to nothing is a
/// failure either way). Also reports a bad file name and a heading
/// duplicated within one file or across two (76bd7ef0 A), plus the
/// letter-set check below -- three more check-only extras `approve` never
/// refuses on, so none of them is a `record_refusals` category. Returns
/// the failures plus the sorted, deduped list of paths still awaiting
/// approval (printed as a summary line by the caller, not a failure by
/// itself).
#[allow(clippy::too_many_arguments)]
pub(super) fn check_plan(
    root: &Path,
    source: &Source,
    records: &[RecordFile],
    led: &Ledger,
    refusals: &[Vec<Refusal>],
    raw_hashes: &HashMap<String, (Vec<String>, String)>,
) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut awaiting: Vec<String> = Vec::new();
    let mut heading_files: HashMap<String, Vec<String>> = HashMap::new();
    let records_by_rel: HashMap<&str, &RecordFile> =
        records.iter().map(|rf| (rf.rel.as_str(), rf)).collect();

    // Naming half of the same decision as the cross-file duplicate-heading
    // check below (76bd7ef0 A: "each file holds one topic and is named by
    // it..."): every file `repo::list_decision_files_via` lists through
    // `source`, by its own base name alone -- checked straight off the
    // listing rather than off `records`, so a file whose name is wrong is
    // still told so even when its content also fails to parse into one.
    for f in repo::list_decision_files_via(root, source) {
        let rel = state::rel_path(root, &f);
        let topic = records_by_rel.get(rel.as_str()).and_then(|rf| rf.topic.as_deref());
        for m in actions::own_name_refusals(&rel, topic) {
            out.push(Failure {
                check: "plan".to_string(),
                message: m,
            });
        }
    }

    for (rf, refs) in records.iter().zip(refusals) {
        let (rel, plan_ok, ctx_ok) = file_approval(rf, raw_hashes, led);
        if !plan_ok || !ctx_ok {
            awaiting.push(rel.clone());
        }
        // The missing-depends-line, entry-resolution, derive-error, and
        // tier-cap failures: `record_refusals`'s `plan` category for this
        // file, computed once in `mod.rs`.
        for r in refs {
            if r.category == "plan" {
                out.push(Failure {
                    check: "plan".to_string(),
                    message: r.message.clone(),
                });
            }
        }

        // A letter in a depends entry's set that the same-file target has no
        // option for (per "Does a depends entry carry the answers it holds
        // under?"): `actions::letter_refusals`, the function `approve` and
        // `adr plan` read too, for every file, approved or not.
        for msg in actions::letter_refusals(&rel, rf, led) {
            out.push(Failure {
                check: "plan".to_string(),
                message: msg,
            });
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
        for m in actions::duplicate_heading_refusals(&rel, &rf.sections) {
            out.push(Failure {
                check: "plan".to_string(),
                message: m,
            });
        }
    }

    let mut headings: Vec<&String> = heading_files.keys().collect();
    headings.sort();
    for heading in headings {
        let mut paths = heading_files[heading].clone();
        paths.sort();
        paths.dedup();
        if paths.len() > 1 {
            out.push(Failure {
                check: "plan".to_string(),
                message: msg::duplicate_heading_across_files(heading, &paths[0], &paths[1]),
            });
        }
    }

    awaiting.sort();
    awaiting.dedup();
    (out, awaiting)
}

/// What differs in `rf` from its latest approval (per "Does approve hash each
/// question on its own?": a removed question takes its hash with it and
/// nothing else moves; check sees which section changed). Returns the
/// headings of the questions whose hash has no `approved` row for the file
/// (new or changed), and the ids of the approved questions no longer in the
/// file and not dropped (struck). `hashes` are the file's question hashes as
/// `raw_hashes` holds them, in section order. Empty when the file was never
/// approved.
pub(super) fn changed_questions(rf: &RecordFile, hashes: &[String], led: &Ledger) -> (Vec<String>, Vec<String>) {
    let rel = rf.rel.as_str();
    let Some(file_row) = led.latest_for_path("approved", rel) else {
        return (Vec::new(), Vec::new());
    };
    let rows = || led.rows.iter().filter(|r| r.event == "approved" && r.detail == rel && r.id != "-");
    let changed: Vec<String> = rf
        .sections
        .iter()
        .zip(hashes)
        .filter(|(sec, h)| !sec.id.as_ref().is_some_and(|id| rows().any(|r| &r.id == id && &r.hash == *h)))
        .map(|(sec, _)| sec.heading_text.clone())
        .collect();
    // The approved set is the latest row of each id, minus the dropped ones;
    // it stands for the last approval only when it derives that file hash.
    let mut latest: Vec<(&str, &str)> = Vec::new();
    for r in rows() {
        match latest.iter_mut().find(|(id, _)| *id == r.id) {
            Some(e) => e.1 = &r.hash,
            None => latest.push((&r.id, &r.hash)),
        }
    }
    let gone = |id: &str, h: &str| led.rows.iter().any(|d| d.event == "dropped" && d.id == id && d.hash == h);
    let in_file = |id: &str| rf.sections.iter().any(|s| s.id.as_deref() == Some(id));
    let approved: Vec<String> = latest.iter().map(|(_, h)| h.to_string()).collect();
    let live: Vec<String> = latest.iter().filter(|(id, h)| !gone(id, h)).map(|(_, h)| h.to_string()).collect();
    let matches = |v: &[String]| record::derive_file_hash(v) == file_row.hash;
    let struck = if matches(&approved) || matches(&live) {
        latest
            .iter()
            .filter(|(id, h)| !in_file(id) && !gone(id, h))
            .map(|(id, _)| id.to_string())
            .collect()
    } else {
        Vec::new()
    };
    (changed, struck)
}
