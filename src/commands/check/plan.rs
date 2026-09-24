use super::{file_approval, Failure};
use crate::depends::{self, DepTarget};
use crate::ledger::Ledger;
use crate::record::{self, AnswerSet, RecordFile};
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

    // Minted in memory (E1.3): a terms file that has never been approved
    // carries no ids on disk yet, and `phrase_lookup` only resolves a term
    // that has one. Without this, every `depends:` entry naming a real but
    // not-yet-approved term would wrongly refuse "which is no term", one
    // line per entry, drowning out the single `terms: adrdom/terms.md not
    // approved` line `check`'s `terms` check already prints.
    let terms_list: Vec<terms::Term> = state::load_terms(root)
        .map(|tf| terms::minted_copy(&tf).terms)
        .unwrap_or_default();
    // Built once for the whole run, not once per `depends:` entry across
    // every file (`resolve_dep` used to rebuild this internally via
    // `terms::resolve_word` on every call).
    let phrase_lookup = terms::phrase_lookup(&terms_list);

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

        // Resolve every entry (8.3/9.1 A), same rules and wording as
        // `approve`: an `Id` entry must be a heading id in this file, a
        // current non-retired term id, or a current decision; a `Text`
        // entry ending in `?` is a heading reference, whose dangling/
        // cycle/self-dep is `derive_tiers`'s single error, computed at
        // parse time into `rf.derive_error`; any other `Text` entry must
        // resolve to exactly one non-retired term. A letter-set suffix, on
        // either form, validates against the target once found (9.1 A
        // point 9: "letter sets validate against the target found by id").
        for sec in &rf.sections {
            for d in &sec.depends {
                match depends::resolve_dep(d, rf, &terms_list, &phrase_lookup, led) {
                    Err(e) => out.push(Failure {
                        check: "plan".to_string(),
                        id: sec.id.clone(),
                        message: depends::dep_refusal_text(&rel, &sec.heading_text, &e),
                    }),
                    Ok(Some(DepTarget::Heading(ti))) => {
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
                    Ok(_) => {}
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
