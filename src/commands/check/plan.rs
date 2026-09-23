use super::{file_approval, Failure};
use crate::ledger::Ledger;
use crate::record::{self, AnswerSet, Dep, RecordFile};
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
                    Dep::Heading { text, set: Some(AnswerSet::Letters(letters)) } => {
                        // 8.7 A, checked here unconditionally (ask.rs only
                        // checks this once the target's tier opens): once
                        // the target already has options, every letter in
                        // the set must be one of them.
                        let target_text = text.trim();
                        if let Some(target) =
                            rf.sections.iter().find(|s| s.heading_text.trim() == target_text)
                        {
                            if !target.options.is_empty() {
                                for l in letters {
                                    if !target.options.iter().any(|o| o.letter == *l) {
                                        out.push(Failure {
                                            check: "plan".to_string(),
                                            id: sec.id.clone(),
                                            message: record::bad_option_letter_text(
                                                &rel,
                                                &sec.heading_text,
                                                &target.heading_text,
                                                *l,
                                            ),
                                        });
                                    }
                                }
                            }
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
                message: record::derive_error_text(&rel, err, None),
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
                message: format!("duplicate heading \"{}\" twice in {}", heading, rel),
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
