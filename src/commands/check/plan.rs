use super::{file_approval, Failure};
use crate::actions::Refusal;
use crate::depends::{self, DepTarget};
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{AnswerSet, RecordFile};
use crate::repo;
use crate::state;
use crate::terms;
use std::collections::HashMap;
use std::path::Path;

/// `plan`: per file, compares the current plan/context hash against the
/// latest `approved`/`context` ledger rows for that path. Also reports the
/// same depends-line/derivation/cap problems `approve` refuses on (8.3/8.4/
/// 8.8, the `plan` category of `record_refusals`, computed once in
/// `mod.rs` and shared with `approve`'s `do_approve_record`, E1.2):
/// unconditionally, regardless of the file's approval state (a derive
/// error, a missing depends line, or an entry that resolves to nothing is a
/// failure either way). Also reports a bad file name and a heading
/// duplicated within one file or across two (76bd7ef0 A), plus the
/// letter-set check below -- three more check-only extras `approve` never
/// refuses on, so none of them is a `record_refusals` category. Returns
/// the failures plus the sorted, deduped list of paths still awaiting
/// approval (printed as a summary line by the caller, not a failure by
/// itself).
pub(super) fn check_plan(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    refusals: &[Vec<Refusal>],
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut awaiting: Vec<String> = Vec::new();
    let mut heading_files: HashMap<String, Vec<String>> = HashMap::new();

    // Naming half of the same decision as the cross-file duplicate-heading
    // check below (76bd7ef0 A: "each file holds one topic and is named by
    // it..."): every file `repo::list_decision_files` lists, by its own
    // base name alone -- checked straight off the listing rather than off
    // `records`, so a file whose name is wrong is still told so even when
    // its content also fails to parse into one.
    for f in repo::list_decision_files(root) {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !repo::valid_topic_file_name(name) {
            out.push(Failure {
                check: "plan".to_string(),
                id: None,
                message: msg::bad_topic_file_name(&state::rel_path(root, &f)),
            });
        }
    }

    for (rf, refs) in records.iter().zip(refusals) {
        let (rel, plan_ok, ctx_ok) = file_approval(root, rf, led);
        if !plan_ok || !ctx_ok {
            awaiting.push(rel.clone());
        }
        if !plan_ok {
            for sec in &rf.sections {
                if !sec.is_skeleton {
                    out.push(Failure {
                        check: "plan".to_string(),
                        id: sec.id.clone(),
                        message: msg::body_under_unapproved_heading(&rel, &sec.heading_text),
                    });
                }
            }
        }

        // The missing-depends-line, entry-resolution, derive-error, and
        // tier-cap failures: `record_refusals`'s `plan` category for this
        // file, computed once in `mod.rs`.
        for r in refs {
            if r.category == "plan" {
                out.push(Failure {
                    check: "plan".to_string(),
                    id: None,
                    message: r.message.clone(),
                });
            }
        }

        // A letter-set suffix on a resolved heading dependency validates
        // against the target once found (9.1 A point 9: "letter sets
        // validate against the target found by id") -- `approve` never
        // checks this today, so it stays a check-only extra rather than a
        // `record_refusals` category.
        for sec in &rf.sections {
            for d in &sec.depends {
                if let Ok(Some(DepTarget::Heading(ti))) =
                    depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, led)
                {
                    if let Some(AnswerSet::Letters(letters)) = d.set() {
                        for msg in
                            depends::bad_letters(&rel, &sec.heading_text, &rf.sections[ti], letters)
                        {
                            out.push(Failure {
                                check: "plan".to_string(),
                                id: sec.id.clone(),
                                message: msg,
                            });
                        }
                    }
                }
            }
        }

        // Same heading twice in one file (8.3-adjacent): the cross-file
        // dedup below only reports a heading once per distinct path, so two
        // identical `### ` headings inside this one file would otherwise
        // never surface (and `derive_tiers`' heading index binds a depends
        // line to the first of the two only, silently).
        let mut local_counts: HashMap<&str, usize> = HashMap::new();
        for sec in &rf.sections {
            *local_counts.entry(sec.heading_text.as_str()).or_insert(0) += 1;
            heading_files
                .entry(sec.heading_text.clone())
                .or_default()
                .push(rel.clone());
        }
        let mut dup_headings: Vec<&str> = local_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(h, _)| h)
            .collect();
        dup_headings.sort();
        for heading in dup_headings {
            out.push(Failure {
                check: "plan".to_string(),
                id: None,
                message: msg::duplicate_heading_in_file(heading, &rel),
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
                id: None,
                message: msg::duplicate_heading_across_files(heading, &paths[0], &paths[1]),
            });
        }
    }

    awaiting.sort();
    awaiting.dedup();
    (out, awaiting)
}
