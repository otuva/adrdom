use super::{file_approval, Failure};
use crate::actions::{self, Refusal};
use crate::depends::{self, DepTarget};
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{AnswerSet, RecordFile};
use crate::repo::{self, Source};
use crate::state;
use crate::terms;
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
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    raw_hashes: &HashMap<String, (String, String)>,
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

        // One line for the file, after its refusals above: a heading added
        // to an approved file puts every body in it under an unapproved
        // plan, and one line for each would bury the refusal that names
        // the agent's move.
        if !plan_ok {
            let bodies: Vec<&str> = rf
                .sections
                .iter()
                .filter(|sec| !sec.is_skeleton)
                .map(|sec| sec.heading_text.as_str())
                .collect();
            match bodies.as_slice() {
                [] => {}
                [one] => out.push(Failure {
                    check: "plan".to_string(),
                    message: msg::body_under_unapproved_heading(&rel, one),
                }),
                many => out.push(Failure {
                    check: "plan".to_string(),
                    message: msg::bodies_under_unapproved_headings(&rel, many.len()),
                }),
            }
        }

        // A letter-set suffix on a resolved dependency validates against
        // the target once found (per "Does a depends entry carry the
        // answers it holds under?") -- `approve` never checks this today,
        // so it stays a check-only extra rather than a `record_refusals`
        // category. A same-file `Heading` target reads its options
        // straight off `rf`; a cross-file `Decision` target (an already-id
        // entry, or a heading-shaped one `normalize_cross_file_deps`
        // resolved above) has its options nowhere in `rf`, so its section
        // is found in `records` instead (`bad_letters_for_decision`).
        for sec in &rf.sections {
            for d in &sec.depends {
                let Some(AnswerSet::Letters(letters)) = d.set() else {
                    continue;
                };
                match depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross_file_headings, led) {
                    Ok(Some(DepTarget::Heading(ti))) => {
                        for msg in
                            depends::bad_letters(&rel, &sec.heading_text, &rf.sections[ti], letters)
                        {
                            out.push(Failure {
                                check: "plan".to_string(),
                                message: msg,
                            });
                        }
                    }
                    Ok(Some(DepTarget::Decision(id))) => {
                        for msg in depends::bad_letters_for_decision(
                            &rel,
                            &sec.heading_text,
                            records,
                            &id,
                            letters,
                        ) {
                            out.push(Failure {
                                check: "plan".to_string(),
                                message: msg,
                            });
                        }
                    }
                    _ => {}
                }
            }
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
