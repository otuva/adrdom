use super::{file_approval, Failure};
use crate::actions::Refusal;
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
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !repo::valid_topic_file_name(name) {
            out.push(Failure {
                check: "plan".to_string(),
                message: msg::bad_topic_file_name(&state::rel_path(root, &f)),
            });
            continue;
        }
        // "is named by it": a well-formed name must also be the file's h1
        // topic, lowercased and hyphenated. A file with no h1 already fails
        // its shape check, so it is skipped here.
        let rel = state::rel_path(root, &f);
        let topic = records_by_rel.get(rel.as_str()).and_then(|rf| rf.topic.as_deref());
        if let Some(topic) = topic {
            let expected = repo::expected_topic_file_stem(topic);
            if name.strip_suffix(".md") != Some(expected.as_str()) {
                out.push(Failure {
                    check: "plan".to_string(),
                    message: msg::topic_file_name_mismatch(&rel, topic, &expected),
                });
            }
        }
    }

    for (rf, refs) in records.iter().zip(refusals) {
        let (rel, plan_ok, ctx_ok) = file_approval(rf, raw_hashes, led);
        if !plan_ok || !ctx_ok {
            awaiting.push(rel.clone());
        }
        if !plan_ok {
            for sec in &rf.sections {
                if !sec.is_skeleton {
                    out.push(Failure {
                        check: "plan".to_string(),
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
                    message: r.message.clone(),
                });
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
                message: msg::duplicate_heading_across_files(heading, &paths[0], &paths[1]),
            });
        }
    }

    awaiting.sort();
    awaiting.dedup();
    (out, awaiting)
}
