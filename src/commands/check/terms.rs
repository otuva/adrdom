use super::Failure;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::rules;
use crate::rules::Class;
use crate::state::{self, rel_path};
use crate::terms;
use std::path::Path;

/// Whether the terms brief is held to the terms file: the file is approved as
/// it stands and every entry has its id. The one gate `check_terms` and the
/// terms brief copy of a satellite share.
pub(super) fn terms_brief_ready(led: &Ledger, tf: &terms::TermsFile) -> bool {
    state::terms_approved(led, tf) && tf.missing_ids().is_empty()
}

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
/// Returns the failures, and whether the terms file is there and differs
/// from its latest approval: a commit waits on it.
pub(super) fn check_terms(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    source: &Source,
    approved_plans: &std::collections::HashSet<String>,
) -> (Vec<Failure>, bool) {
    let mut out = Vec::new();
    let has_records = !repo::list_decision_files_via(root, source).is_empty();

    let Some(content) = source.read(root, repo::TERMS_PATH) else {
        if has_records {
            out.push(Failure::said(Class::Terms, msg::terms_missing()));
        }
        return (out, false);
    };

    let lines = record::split_lines(&content);
    let tf = terms::parse(&lines);

    // A missing id is not a shape error the agent can fix: only approve
    // mints ids. A terms file with one is never approved either (its hash
    // can never match a `terms` ledger row `approve` wrote), so it is
    // caught by the same not-approved gate below rather than its own
    // separate check.
    let ready = terms_brief_ready(led, &tf);

    let approved = state::terms_approved(led, &tf);

    // Per "Does the tool stop while the terms file waits for approval?"
    // (aa475aa3 A, supersedes "Does check fail a terms file that differs
    // from its latest approval?" and "Does the owner approve the terms file
    // as a whole?"): an unapproved (or shape-broken-on-ids) terms.md fails
    // outright. Nothing below the gate -- section terms-line, term
    // terms-line -- runs while it does, since all of
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
    // A banned word in the file is printed by `check_banned`; approve
    // refuses on it all the same, so the owner's approve is not named
    // beside it.
    let refused =
        !refusals.is_empty() || !rules::terms_banned_hits(root, &minted, source).is_empty();
    for r in refusals {
        out.push(r);
    }

    if !ready {
        // The owner's move is named only when `approve` would take the
        // file: with a refusal above, the agent's move comes first.
        // One finding per entry that waits, each with its own approve (per
        // "What does approve take for a terms file holding several entries
        // that wait?", da0dc8b1 A); the file as a whole when none does.
        if !refused {
            let waiting = crate::state::waiting_terms(led, &tf);
            if waiting.is_empty() {
                out.push(Failure::said(
                    Class::Terms,
                    msg::terms_file_not_approved(crate::status::terms_file_status(led, &tf)),
                ));
            }
            for (name, status) in &waiting {
                out.push(Failure::said(
                    Class::Terms,
                    msg::terms_entry_waits(None, name, *status),
                ));
            }
        }
        return (out, !approved);
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
        out.push(Failure::said(
            Class::Terms,
            if on_disk {
                msg::terms_brief_not_staged()
            } else if source.exists(root, repo::TERMS_BRIEF_PATH) {
                msg::terms_brief_differs()
            } else {
                msg::terms_brief_missing()
            },
        ));
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
            // or only asked (still open, `approve` can rewrite it): the message
            // functions below need this to pick a legal next move.
            let answered = st == Some("answered");
            let Some(tl) = sec.terms_line else {
                out.push(Failure::said(
                    Class::Terms,
                    msg::section_no_terms_line(&rel, &sec.heading_text, answered),
                ));
                continue;
            };
            match terms::validate_pairs(rf.lines[tl].trim_end(), &tf) {
                Err(_) => out.push(Failure::said(
                    Class::Terms,
                    msg::section_terms_line_malformed(&rel, &sec.heading_text, answered),
                )),
                Ok(problems) => {
                    for p in problems {
                        match p.kind {
                            terms::ProblemKind::Gone => out.push(Failure::said(
                                Class::Terms,
                                msg::section_term_not_found(
                                    &rel,
                                    &sec.heading_text,
                                    &p.word,
                                    answered,
                                ),
                            )),
                            terms::ProblemKind::Retired(name) => out.push(Failure::said(
                                Class::Terms,
                                msg::section_term_retired(&rel, &sec.heading_text, &name, answered),
                            )),
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
            Err(_) => out.push(Failure::said(
                Class::Terms,
                msg::term_definition_terms_line_malformed(&t.name),
            )),
            Ok(problems) => {
                for p in problems {
                    match p.kind {
                        terms::ProblemKind::Gone => out.push(Failure::said(
                            Class::Terms,
                            msg::term_definition_term_not_found(&t.name, &p.word),
                        )),
                        terms::ProblemKind::Retired(name) => out.push(Failure::said(
                            Class::Terms,
                            msg::term_definition_term_retired(&t.name, &name),
                        )),
                    }
                }
            }
        }
    }

    (out, false)
}
