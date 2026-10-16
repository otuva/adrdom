//! Rules on the terms file.
use super::*;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::repo;
use crate::state;
use crate::terms;
use std::path::Path;

/// A finding about the terms file from what a message function said.
fn terms_finding(said: msg::Said) -> Finding {
    Finding::said(Class::Terms, said).about(Subject::TermsFile)
}

/// Every refusal `adrdom/terms.md` earns under steps 1, 3, 4 and 5 of
/// `do_approve_terms` (shape errors, circular definitions, a dependent's
/// pairing that no longer holds, banned words in the file's own text),
/// plus a definition running past two sentences
/// (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76, not one of the numbered steps
/// above), collected all at once rather than returned at the first --
/// `do_approve_terms` prints every one and writes nothing on any refusal;
/// `check` reports the same list, each line prefixed `terms: ` instead of
/// `approve: `, minus the approval-state lines that only make sense after a
/// real write (`already approved`/`approved`, the definition-changed
/// notes), which stay `check`'s own.
///
/// `tf` must already have every id minted in memory (step 2, see
/// `terms::mint_ids`/`terms::minted_copy`): `do_approve_terms` mints
/// straight into the file it is about to write before calling this; `check`
/// -- which never writes -- calls this on a `terms::minted_copy` instead.
/// Either way, a cycle or a moved/retired dependent's pairing in a terms
/// file that has never been approved, and so carries no ids on disk, is
/// caught the same way both callers look for it (`terms::cycles` and
/// `terms::phrase_lookup` only see a term that has an id).
///
/// Returns every refusal message (`adrdom/terms.md: ...`, no caller
/// prefix) plus each term's freshly resolved definition pairs
/// (`terms::definition_pairs`, in `tf.terms` order), computed once here
/// and reused by `do_approve_terms`'s write step so it is never resolved
/// twice.
///
/// `records` is every topic file in the repo, already parsed by the
/// caller: `do_approve_terms` parses them once itself and reuses
/// that same parse for its step-7 report; `check` passes the same
/// `RecordFile`s it built for its own record checks. Neither re-reads a
/// topic file from disk just to compute a dependent's terms line here.
///
/// `source` is who the ban list (step 5) is read through: `approve` passes
/// `Source::WorkingTree` (it always reads `adrdom/terms.md` straight off
/// disk), `check` passes whichever source it is judging (per "Does check at
/// a commit read the staged files?"), so a staged edit to the ban list is
/// judged, not whatever is left on disk. `scan_banned` is false only for
/// `check`'s own call: `check_banned` already scans `adrdom/terms.md` as a
/// tracked file, so step 5 would otherwise report the same hit twice, once
/// prefixed `banned:` and once prefixed `terms:`; `approve` (the one caller
/// that never runs `check_banned` itself) always passes `true`, since step 5
/// is its only banned-word gate on the file.
pub fn terms_refusals(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
    source: &repo::Source,
    scan_banned: bool,
    approved_plans: &std::collections::HashSet<String>,
) -> (Vec<Finding>, Vec<Vec<(String, String)>>) {
    let mut refusals: Vec<Finding> = Vec::new();

    // Step 1 (shape): "missing id" cannot appear here, since `tf` is
    // already minted.
    for e in &tf.errors {
        refusals.push(terms_finding(msg::terms_shape_error(e)));
    }

    // Step 3: definition pairings; a self-pairing or any cycle refuses.
    // Every cycle is reported, not just the first.
    for cycle in terms::cycles(tf) {
        refusals.push(terms_finding(msg::circular_definition(&cycle)));
    }

    // Fresh definition pairs for every term (never re-derived from a stale
    // terms line in the file). Computed once and reused for the dependents
    // check below and, by the caller, for the write step.
    let pairs_per_entry: Vec<Vec<(String, String)>> = (0..tf.terms.len())
        .map(|i| terms::definition_pairs(tf, i))
        .collect();

    // Every current decision's terms line, every current decision's
    // `depends:` line (once approved, an id -- possibly a term's), and
    // every non-retired term's recorded pairing, is a dependent -- the
    // widest reach of `state::dependent_entries`' three readers.
    // Asked questions and not-yet-asked headings of approved plans (in the
    // files of `approved_plans`, read as written on disk) name terms too.
    let mut dependents: Vec<state::DependentEntry> =
        state::dependent_entries(records, led, tf, true, true);
    dependents.extend(state::unsettled_entries(records, led, tf, approved_plans));

    // Step 4: every pair on a dependent must still hold in the new file.
    // `lookup` answers "what does this exact word/phrase resolve to" for
    // every dependent pair, built once rather than once per pair.
    let lookup = terms::phrase_lookup(&tf.terms);
    for dep in &dependents {
        let label = dep.source.label();
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf
                .terms
                .iter()
                .find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                let word = dep.from_word.then_some(word.as_str());
                refusals.push(terms_finding(msg::term_removed_but_carried(
                    uuid,
                    word,
                    &label,
                    dep.source.decision_ref(),
                )));
                continue;
            };
            if target.retired {
                // A decision dependent: the agent's move is to write a new
                // question that supersedes it, not the owner's to make on
                // the spot. A term dependent (its own definition still
                // carries the retired word) is never superseded (per "What
                // holds a retire of a term?"): the agent rewords the
                // definition and the owner re-approves.
                let user = match &dep.source {
                    state::DependentSource::Decision { id, heading, .. } => {
                        msg::RetiredUser::Decision(id, heading)
                    }
                    state::DependentSource::Asked { rel, .. } => msg::RetiredUser::Asked(rel),
                    state::DependentSource::PlanHeading { rel, .. } => {
                        msg::RetiredUser::PlanHeading(rel)
                    }
                    state::DependentSource::Term { .. } => msg::RetiredUser::Definition,
                };
                refusals.push(terms_finding(msg::term_retired_but_carried(
                    &target.name,
                    &label,
                    user,
                )));
                continue;
            }
            // A `depends:`-line dependent's pair is an id read straight off
            // the entry, never a resolved word -- there is no word to
            // drift, only the target's removal/retirement above, so this
            // check is skipped for it (`from_word` is false).
            let is_decision_or_term = matches!(
                dep.source,
                state::DependentSource::Decision { .. } | state::DependentSource::Term { .. }
            );
            if dep.from_word && is_decision_or_term {
                let still_resolves = lookup.get(word) == Some(uuid);
                if !still_resolves {
                    // A decision dependent here is always current, since
                    // only `is_current` sections become one above -- so it
                    // can never be re-asked (per "Does a decision change
                    // after it is answered?"; "Does a refusal name the
                    // command that clears it?" means a refusal about an
                    // answered question never tells the agent to re-ask it,
                    // since that can't clear it). What moved is
                    // adrdom/terms.md, not the decision's own text, so the
                    // fix is there: restore the wording so the word
                    // resolves to this term again, or, when the move is
                    // meant, supersede the decision with a new question
                    // that pairs the new wording. A term dependent (its own
                    // definition still carries the drifted word) is
                    // reworded by the agent and the owner re-approves, same
                    // as the retired case above.
                    refusals.push(terms_finding(msg::word_moved_off_term(
                        word,
                        &target.name,
                        &label,
                        dep.source.decision_id(),
                    )));
                }
            }
        }
    }

    // Step 5: banned hits in terms.md's own text, from the single ban list
    // (skipped for `check`'s call -- see `scan_banned` above -- since
    // `check_banned` already scans this same file as a tracked file).
    if scan_banned {
        refusals.extend(terms_banned_hits(root, tf, source));
    }

    // Every term's own definition is one or two sentences
    // (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76; not one of the spec's
    // numbered steps). Checked on every entry, retired or not, the same
    // way the shape checks in `terms::parse` run on every entry regardless
    // of `retired`.
    for t in &tf.terms {
        let count = terms::sentence_count(&t.definition);
        if count > 2 {
            refusals.push(terms_finding(msg::definition_too_long(&t.name, count)));
        }
    }

    (refusals, pairs_per_entry)
}

/// The banned words `adrdom/terms.md` holds, each by the line the file as
/// written holds it on: what `approve` refuses the terms file on, and what
/// `check` reads to know whether `approve` would take it (`check_banned`
/// names the hit itself, as it does in every tracked file).
pub fn terms_banned_hits(
    root: &Path,
    tf: &terms::TermsFile,
    source: &repo::Source,
) -> Vec<Finding> {
    let banned_entries = crate::banned::read_entries_via(root, source);
    if banned_entries.is_empty() {
        return Vec::new();
    }
    let hash_map: std::collections::HashMap<String, Option<String>> =
        banned_entries.into_iter().collect();
    // The file as written, not the minted copy: an entry minted an id gains
    // a line, and a hit names the line the file holds.
    let text = source
        .read(root, crate::repo::TERMS_PATH)
        .unwrap_or_else(|| tf.lines.join("\n"));
    crate::banned::hits(&text, &hash_map, &tf.terms)
        .into_iter()
        .map(|hit| {
            terms_finding(msg::terms_banned_word(
                hit.line,
                &msg::term_suffix(hit.term.as_deref()),
            ))
        })
        .collect()
}
