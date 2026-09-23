use super::{file_approval, Failure};
use crate::ledger::Ledger;
use crate::record::{self, Dep, DeriveError, RecordFile};
use crate::state;
use crate::terms;
use std::collections::HashMap;
use std::path::Path;

/// `plan`: per file, compares the current plan/context hash against the
/// latest `approved`/`context` ledger rows for that path. Also reports the
/// same depends-line/derivation/cap problems `approve` refuses on (8.3/8.4/
/// 8.8): unconditionally, regardless of the file's approval state (a derive
/// error, a missing depends line, or an entry that resolves to nothing is a
/// failure either way). Returns the failures plus the sorted, deduped list
/// of paths still awaiting approval (printed as a summary line by the
/// caller, not a failure by itself).
pub(super) fn check_plan(root: &Path, records: &[RecordFile], led: &Ledger) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut awaiting: Vec<String> = Vec::new();
    let mut heading_files: HashMap<String, Vec<String>> = HashMap::new();

    let terms_list: Vec<terms::Term> = state::load_terms(root).map(|tf| tf.terms).unwrap_or_default();

    for rf in records {
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
                        message: format!(
                            "{}: body under unapproved heading \"{}\"",
                            rel, sec.heading_text
                        ),
                    });
                }
            }
        }

        // 8.3/8.6 B: every section, skeleton or not, needs at least one
        // `depends:` line naming what the question stands on.
        for sec in &rf.sections {
            if sec.depends.is_empty() {
                out.push(Failure {
                    check: "plan".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: \"{}\" has no depends line", rel, sec.heading_text),
                });
            }
        }

        // Resolve every entry (8.3 A), same rules and wording as `approve`:
        // a term entry must resolve to exactly one non-retired term; a
        // decision entry must be a current decision (latest state
        // `answered`); a heading entry's dangling/cycle/self-dep is
        // `derive_tiers`'s single error, computed at parse time into
        // `rf.derive_error`.
        for sec in &rf.sections {
            for d in &sec.depends {
                match d {
                    Dep::Term(w) => {
                        let pairs = terms::resolve(w, &terms_list);
                        let resolved_ok = pairs.len() == 1
                            && terms_list
                                .iter()
                                .find(|t| t.id.as_deref() == Some(pairs[0].1.as_str()))
                                .map(|t| !t.retired)
                                .unwrap_or(false);
                        if !resolved_ok {
                            out.push(Failure {
                                check: "plan".to_string(),
                                id: sec.id.clone(),
                                message: format!(
                                    "{}: \"{}\" depends on \"{}\" which is no term; agent adds the term or names another",
                                    rel, sec.heading_text, w
                                ),
                            });
                        }
                    }
                    Dep::Decision(uuid) => {
                        if !state::is_current(led, uuid) {
                            out.push(Failure {
                                check: "plan".to_string(),
                                id: sec.id.clone(),
                                message: format!(
                                    "{}: \"{}\" depends on {} which is not a current decision; agent names a current decision or removes the line",
                                    rel, sec.heading_text, uuid
                                ),
                            });
                        }
                    }
                    Dep::Heading { .. } => {}
                }
            }
        }
        if let Some(err) = &rf.derive_error {
            out.push(Failure {
                check: "plan".to_string(),
                id: None,
                message: derive_error_message(&rel, err),
            });
        } else if let Some(tiers) = &rf.tiers {
            // Cap (8.8 A): no derived tier may hold more than `TIER_CAP`
            // questions.
            if let Some((tier, count)) = record::over_cap(tiers) {
                out.push(Failure {
                    check: "plan".to_string(),
                    id: None,
                    message: format!(
                        "{}: tier {} holds {} questions, the cap is {}",
                        rel, tier, count, record::TIER_CAP
                    ),
                });
            }
        }

        for sec in &rf.sections {
            heading_files
                .entry(sec.heading_text.clone())
                .or_default()
                .push(rel.clone());
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
                message: format!(
                    "duplicate heading \"{}\" in {} and {}",
                    heading, paths[0], paths[1]
                ),
            });
        }
    }

    awaiting.sort();
    awaiting.dedup();
    (out, awaiting)
}

/// The rest of a derive-error failure message after `"<rel>: "` (spec §6:
/// "dangling/cycle/no-term/not-current-decision as in approve"), matching
/// `do_approve_record`'s wording so the agent sees the same fix either way.
fn derive_error_message(rel: &str, err: &DeriveError) -> String {
    match err {
        DeriveError::Dangling { section, entry } => format!(
            "{}: \"{}\" depends on \"{}\" which is no heading in this file; agent adds the heading or fixes the text",
            rel, section, entry
        ),
        DeriveError::Cycle(names) => {
            let mut chain: Vec<String> = names.clone();
            if let Some(first) = names.first() {
                chain.push(first.clone());
            }
            format!(
                "{}: cycle: {}; agent breaks it",
                rel,
                chain.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(" -> ")
            )
        }
        DeriveError::SelfDep(h) => format!(
            "{}: \"{}\" depends on itself; agent removes the line or names something else",
            rel, h
        ),
    }
}
