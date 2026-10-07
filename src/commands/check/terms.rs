use super::Failure;
use crate::ledger::Ledger;
use crate::rules;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::state::{self, rel_path};
use crate::terms;
use std::collections::HashSet;
use std::path::Path;

/// `terms`: shape/approval failures on `adrdom/terms.md` (read through
/// `source` -- the index when `check` is judging a commit, the working
/// tree otherwise, per "Does check at a commit read the staged files?").
/// Missing entirely is a failure once any topic file exists, and a
/// note otherwise. Present but not approved at its current hash is always a
/// failure (per "Does the tool stop while the terms file waits for
/// approval?", aa475aa3 A): the refusals `approve` would give (shape,
/// circular definition, dependent pairing), or, with none, the owner's
/// move. Every
/// asked/answered section's terms line, and every non-retired term's own
/// terms line, is checked for an existing, non-retired uuid on each pair --
/// never by re-resolving the underlying text. Circular definitions fail.
/// Returns the failures and any notes ("paired by nothing").
pub(super) fn check_terms(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    source: &Source,
    approved_plans: &std::collections::HashSet<String>,
) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let has_records = !repo::list_decision_files_via(root, source).is_empty();

    let Some(content) = source.read(root, repo::TERMS_PATH) else {
        if has_records {
            out.push(Failure {
                check: "terms".to_string(),
                message: msg::terms_missing(),
            });
        }
        return (out, notes);
    };

    let lines = record::split_lines(&content);
    let tf = terms::parse(&lines);

    // A missing id is not a shape error the agent can fix: only approve
    // mints ids. A terms file with one is never approved either (its hash
    // can never match a `terms` ledger row `approve` wrote), so it is
    // caught by the same not-approved gate below rather than its own
    // separate check.
    let missing_ids = !tf.missing_ids().is_empty();

    let approved = state::terms_approved(led, &tf);

    // Per "Does the tool stop while the terms file waits for approval?"
    // (aa475aa3 A, supersedes "Does check fail a terms file that differs
    // from its latest approval?" and "Does the owner approve the terms file
    // as a whole?"): an unapproved (or shape-broken-on-ids) terms.md fails
    // outright. Nothing below the gate -- section terms-line, term
    // terms-line, "paired by nothing" -- runs while it does, since all of
    // it reads the file as approved.
    //
    // Shared with approve: every shape, circular-definition,
    // dependent-pairing, and banned-word refusal `adrdom/terms.md` earns,
    // computed on a copy with every id minted in memory so a cycle or a
    // dependent's pairing in a terms file that has never been approved --
    // and so has no ids on disk -- is caught the same way `approve` catches
    // it (`terms::cycles` and `terms::phrase_lookup` only see a term that
    // has an id). It runs on an unapproved file too: the agent reads here
    // what `approve` would refuse, before the owner runs it.
    // `scan_banned: false` -- `check_banned` already scans `adrdom/terms.md`
    // as a tracked file, so step 5 would otherwise report the same hit
    // twice (once prefixed `banned:`, once prefixed `terms:`).
    let minted = terms::minted_copy(&tf);
    let (refusals, _pairs) =
        rules::terms_refusals(root, records, &minted, led, source, false, approved_plans);
    let refused = !refusals.is_empty();
    for r in refusals {
        out.push(Failure {
            check: "terms".to_string(),
            message: r,
        });
    }

    if !approved || missing_ids {
        // The owner's move is named only when `approve` would take the
        // file: with a refusal above, the agent's move comes first.
        if !refused {
            out.push(Failure {
                check: "terms".to_string(),
                message: msg::terms_file_not_approved(crate::status::terms_file_status(led, &tf)),
            });
        }
        return (out, notes);
    }

    // Per "Does check hold the terms brief to the terms file?" (ec3f875c
    // A): the terms brief is built again from the terms file and compared
    // with the file this source holds. When only the working tree holds
    // the right text at a commit, the move is the agent's: stage the file.
    let want = terms::brief_text(&tf);
    let matches = |got: Option<String>| {
        got.is_some_and(|g| {
            crate::text::lines_match_trimmed(
                &g.lines().collect::<Vec<_>>(),
                &want.lines().collect::<Vec<_>>(),
            )
        })
    };
    if !matches(source.read(root, repo::TERMS_BRIEF_PATH)) {
        let on_disk = *source == Source::Index
            && matches(Source::WorkingTree.read(root, repo::TERMS_BRIEF_PATH));
        out.push(Failure {
            check: "terms".to_string(),
            message: if on_disk {
                msg::terms_brief_not_staged()
            } else if source.exists(root, repo::TERMS_BRIEF_PATH) {
                msg::terms_brief_differs()
            } else {
                msg::terms_brief_missing()
            },
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
            // Whether this section is already answered (frozen, never
            // re-asked -- "Does a decision change after it is answered?")
            // or only asked (still open, `ask` can rewrite it): the message
            // functions below need this to pick a legal next move.
            let answered = st == Some("answered");
            let Some(tl) = sec.terms_line else {
                out.push(Failure {
                    check: "terms".to_string(),
                    message: msg::section_no_terms_line(&rel, &sec.heading_text, id, answered),
                });
                continue;
            };
            match terms::validate_pairs(rf.lines[tl].trim_end(), &tf) {
                Err(_) => out.push(Failure {
                    check: "terms".to_string(),
                    message: msg::section_terms_line_malformed(&rel, &sec.heading_text, id, answered),
                }),
                Ok(problems) => {
                    for p in problems {
                        match p.kind {
                            terms::ProblemKind::Gone => out.push(Failure {
                                check: "terms".to_string(),
                                message: msg::section_term_not_found(
                                    &rel,
                                    &sec.heading_text,
                                    &p.uuid,
                                    &p.word,
                                    id,
                                    answered,
                                ),
                            }),
                            terms::ProblemKind::Retired(name) => out.push(Failure {
                                check: "terms".to_string(),
                                message: msg::section_term_retired(&rel, &sec.heading_text, &name, id, answered),
                            }),
                        }
                    }
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
        match terms::validate_pairs(tl, &tf) {
            Err(_) => out.push(Failure {
                check: "terms".to_string(),
                message: msg::term_definition_terms_line_malformed(&t.name),
            }),
            Ok(problems) => {
                for p in problems {
                    match p.kind {
                        terms::ProblemKind::Gone => out.push(Failure {
                            check: "terms".to_string(),
                            message: msg::term_definition_term_not_found(&t.name, &p.uuid, &p.word),
                        }),
                        terms::ProblemKind::Retired(name) => out.push(Failure {
                            check: "terms".to_string(),
                            message: msg::term_definition_term_retired(&t.name, &name),
                        }),
                    }
                }
            }
        }
    }

    // Note (never a failure): every non-retired term that no current
    // decision's terms line, and no non-retired term's own terms line,
    // pairs against (the narrower, non-transitive reach: no `depends:`
    // ids, since a bare `depends:` entry is not a "pairing").
    let used: HashSet<String> = state::dependent_entries(records, led, &tf, false, true)
        .into_iter()
        .flat_map(|dep| dep.pairs.into_iter().map(|(_, uuid)| uuid))
        .collect();
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

    (out, notes)
}

