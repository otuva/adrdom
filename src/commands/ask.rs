use crate::actions;
use crate::banned;
use crate::depends;
use crate::ledger;
use crate::msg::{self, Actor, Next};
use crate::record;
use crate::repo;
use crate::state::{self, TermsLoad};
use crate::tells;
use crate::terms;
use crate::view;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn run(record_path: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("ask", Actor::Agent));
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let rel = state::rel_path(&root, &record_path);

    // Refusals 1-3: adrdom/terms.md missing, unreadable/shape errors, or
    // not approved. These come before anything about the topic file itself is
    // even loaded, since none of them depend on it.
    let tf = match state::load_terms_checked(&root) {
        TermsLoad::Missing => {
            eprintln!("ask: {}", msg::terms_missing());
            return 1;
        }
        TermsLoad::LoadError(e) => {
            eprintln!(
                "ask: {}",
                msg::terms_load_error(&e, Next::agent("fixes adrdom/terms.md and retries"))
            );
            return 1;
        }
        TermsLoad::Ok(tf) => {
            if !tf.errors.is_empty() {
                for e in &tf.errors {
                    eprintln!("ask: {}", msg::terms_shape_error(e));
                }
                return 1;
            }
            tf
        }
    };
    if !state::terms_approved(&led, &tf) {
        eprintln!("ask: {}", msg::terms_not_approved(&rel));
        return 1;
    }
    let terms_list: Vec<terms::Term> = tf.terms.clone();

    let mut rf = match record::load(&record_path) {
        Ok(rf) => rf,
        Err(e) => {
            eprintln!(
                "ask: {}",
                msg::read_failed(
                    &record_path.display().to_string(),
                    e,
                    Next::agent("checks the error and retries"),
                )
            );
            return 1;
        }
    };

    // Shape and parse errors refuse first, before anything else is even
    // considered: the same "parse" category `approve`/`check` refuse on
    // (an option with no decision sentence or more than one, a duplicate id
    // line, an answer naming a missing option -- used to be a parse error
    // only `check` read; `record_refusals` is now the one shared validation
    // all three commands read it through).
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    // Every current decision's heading text, repo-wide: built once here,
    // the same map `approve`/`check` build, and passed to the normalize
    // pass below and to every `resolve_dep` call `ask` makes through
    // `record_refusals`. `ask` has nothing else loaded repo-wide (only
    // `rf`, the one file it was asked about), so this is the one caller
    // that still has to load every topic file for it.
    let all_records: Vec<record::RecordFile> = record::load_all(&root);
    let cross_file_headings = depends::cross_file_heading_index(&all_records, &led);

    // Plan/context approval state must be read off `rf` exactly as loaded,
    // before the normalize pass below mutates its parsed `depends:`
    // entries in memory (per "..."): captured here, ahead of that
    // mutation, and used below via `state::plan_approved_hash`/
    // `context_approved_hash` instead of recomputing from the
    // (by-then-normalized) `rf`.
    let raw_plan_hash = record::plan_hash(&rf);
    let raw_context_hash = record::context_hash(&rf);

    // Same pre-pass `approve`/`check` run before reading `rf.tiers`/
    // `rf.derive_error` (`depends::normalize_cross_file_deps`): a
    // heading-shaped `depends:` entry naming a current decision's heading
    // in another topic file is rewritten in memory to that decision's id
    // and the tiers re-derived, so a file with such an entry never trips
    // the derive-error refusal below just because it hasn't been approved
    // yet. An entry naming a current decision in more than one other file
    // becomes `rf.derive_error` instead (`DeriveError::AmbiguousHeading`),
    // checked immediately below, before the plan-approved gate -- exactly
    // as `approve` refuses on it, and ahead of a "plan not approved"
    // refusal that would otherwise fire first on an unapproved file.
    depends::normalize_cross_file_deps(&mut rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
    if let Some(err @ record::DeriveError::AmbiguousHeading { .. }) = &rf.derive_error {
        eprintln!("ask: {}", msg::derive_error(&rel, err));
        return 1;
    }

    let shape_msgs: Vec<String> =
        actions::record_refusals(&rel, &rf, &terms_list, &phrase_lookup, &cross_file_headings, &led)
            .into_iter()
            .filter(|r| r.category == "parse")
            .map(|r| format!("ask: {}", r.message))
            .collect();
    if !shape_msgs.is_empty() {
        for m in &shape_msgs {
            eprintln!("{}", m);
        }
        return 1;
    }

    // Refuse (and change nothing) if any answered or refused section was
    // edited since it was answered/refused: the same comparison check's
    // `answers.rs` makes (every ledger row for the id at that event, not
    // only the latest, since a genuine re-answer at the same letter after a
    // harmless edit adds a second row rather than rewriting the first),
    // shared through `state::closed_match`.
    for sec in &rf.sections {
        let Some(id) = &sec.id else { continue };
        if sec.answer_line.is_none() {
            continue;
        }
        let cur = record::section_hash(&rf.lines, sec);
        if let state::ClosedMatch::Edited = state::closed_match(&led, &cur, sec, id) {
            if sec.is_refused() {
                eprintln!(
                    "ask: {}",
                    msg::edited_after_refused(&record_path.display().to_string(), &sec.heading_text)
                );
            } else {
                eprintln!(
                    "ask: {}",
                    msg::edited_after_answered(&record_path.display().to_string(), &sec.heading_text)
                );
            }
            return 1;
        }
    }

    // Refusal 4: plan / context must be approved before ask will touch the
    // file at all -- checked against the hashes captured before the
    // normalize pass above, never the normalized `rf`.
    if !state::plan_approved_hash(&led, &rel, &raw_plan_hash) {
        eprintln!("ask: {}", msg::plan_not_approved(&rel));
        return 1;
    }
    if !state::context_approved_hash(&led, &rel, &raw_context_hash) {
        eprintln!("ask: {}", msg::context_not_approved(&rel));
        return 1;
    }

    // Sections that are about to be newly asked or re-asked this run: no
    // id yet, or an id whose current hash differs from its latest `asked`
    // row. Computed against the file exactly as loaded, before any
    // mutation, so the gates below (and any refusal) see the same file
    // the owner is looking at.
    let candidate: Vec<bool> = rf
        .sections
        .iter()
        .map(|sec| {
            if sec.is_skeleton {
                return false;
            }
            match &sec.id {
                None => true,
                Some(id) => match led.latest_state(id) {
                    None => true,
                    Some(row) if row.event == "asked" => {
                        record::section_hash(&rf.lines, sec) != row.hash
                    }
                    _ => false,
                },
            }
        })
        .collect();

    // Derive-error refusal (8.4/8.7): a derivation failure means `approve`
    // would also refuse on this same file, so the fix has to happen before
    // either command can move: point the owner at the topic file first, the
    // re-approval second.
    if let Some(err) = &rf.derive_error {
        eprintln!("ask: {}", msg::derive_error_ask(&rel, err));
        return 1;
    }

    // Cap refusal (per "Does the tool cap the questions in a tier?"): any
    // derived tier over `TIER_CAP` live questions,
    // file-wide (mirrors approve's cap check, prefixed `ask:`). Dead
    // (refused/superseded/withdrawn) questions don't count.
    if let Some(tiers) = &rf.tiers {
        let live = state::live_tiers(&led, &rf, tiers);
        if let Some((n, m)) = record::over_cap(&live) {
            eprintln!("ask: {}", msg::tier_cap_exceeded(&rel, n, m, record::TIER_CAP));
            return 1;
        }
    }

    // The open tier (per "Does the tool derive tiers from the depends
    // lines?"): the lowest tier holding an unresolved section
    // (`state::open_tier`; a refused question counts as unresolved, and so
    // does a skeleton heading with no answer -- per "Does the owner answer
    // a question with a refusal?" and "Does a topic file close?"). Every
    // candidate has no letter answer by construction, so whenever any
    // candidate exists, it names a tier in this set and `open_tier` is
    // never `None`.
    let open_tier: Option<usize> = state::open_tier(&rf);

    // Answer sets mandatory (per "Does a depends entry carry the answers
    // it holds under?"): before asking any candidate in the
    // open tier, every section in the file with a heading dep pointing at
    // an open-tier section must carry an explicit set on that entry, and
    // every set's letters must be options the target actually has. Runs
    // once, ahead of the per-candidate loop, and only when there is
    // something new to ask in the open tier this run.
    if let Some(n) = open_tier {
        let asking_in_open_tier = rf
            .sections
            .iter()
            .enumerate()
            .any(|(i, _)| candidate[i] && rf.tier_of(i) == Some(n));
        if asking_in_open_tier {
            for (ti, target) in rf.sections.iter().enumerate() {
                if rf.tier_of(ti) != Some(n) {
                    continue;
                }
                for (di, set) in rf.dependents_of(ti) {
                    let dependent = &rf.sections[di];
                    match set {
                        None => {
                            eprintln!(
                                "ask: {}",
                                msg::missing_answer_set(&rel, &dependent.heading_text, &target.heading_text)
                            );
                            return 1;
                        }
                        Some(record::AnswerSet::Any) => {}
                        Some(record::AnswerSet::Letters(letters)) => {
                            if let Some(m) =
                                depends::bad_letters(&rel, &dependent.heading_text, target, letters)
                                    .into_iter()
                                    .next()
                            {
                                eprintln!("ask: {}", m);
                                return 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let banned_path = root.join(banned::BAN_LIST);
    let banned_entries: HashMap<String, Option<String>> =
        banned::read_entries(&banned_path).into_iter().collect();

    // Per "Does a depends entry carry the answers it holds under?": every
    // moot heading dep in the file, gated on the ledger's `qstate` rather
    // than just the file's letter (shared with `check`/the page through
    // `view::moot_pairs`, so the two can never diverge on what counts as
    // moot). Computed once, ahead of the per-candidate loop.
    let moot_pairs = view::moot_pairs(&rf, &led);

    for (i, sec) in rf.sections.iter().enumerate() {
        if !candidate[i] {
            continue;
        }

        // A candidate that directly stands on a refused question refuses
        // with the specific dependency named, ahead of the generic tier
        // rule below (whose "waiting on" blocker, chosen by file order
        // within the open tier, need not be this candidate's own
        // dependency even when the generic rule would also fire).
        for d in &sec.depends {
            if let Some(ti) = rf.dep_target(d) {
                let target = &rf.sections[ti];
                if target.is_refused() {
                    eprintln!(
                        "ask: {}",
                        msg::depends_on_refused(&rel, &sec.heading_text, &target.heading_text)
                    );
                    return 1;
                }
            }
        }

        // Supersedes target must be current (per "Does one decision
        // supersede several?", owner note 2026-09-24): the supersedes
        // line freezes at ask, so a
        // candidate naming a target that is not current right now -- never
        // answered, refused, withdrawn, or already superseded -- refuses
        // here; `answer` later accepts a target that was superseded after
        // this question's own `asked` row (a split), but never one that was
        // already stale before it was ever asked.
        for old in &sec.supersedes {
            if !state::is_current(&led, old) {
                eprintln!("ask: {}", msg::ask_supersedes_not_current(&rel, &sec.heading_text, old));
                return 1;
            }
        }

        // Tier rule (per "Does the tool derive tiers from the depends
        // lines?"): a candidate outside the open tier refuses, naming the
        // tier's blocker (`state::tier_blocker`: the first non-skeleton
        // unresolved section in the open tier, by file order, or the
        // tier's own skeleton heading when it holds nothing else) and
        // whether that blocker is refused.
        let cand_tier = rf.tier_of(i).unwrap_or(1);
        if let Some(open_n) = open_tier {
            if cand_tier != open_n {
                if let Some(blocker) = state::tier_blocker(&rf, open_n) {
                    // The next move depends on where the blocker stands:
                    // refused -> the plan changes; asked -> the owner
                    // answers it (by id@version, the form `answer`
                    // takes); anything else (a skeleton, or a body `ask`
                    // has not run on) -> the agent asks it first.
                    let blocker_state = blocker
                        .id
                        .as_deref()
                        .and_then(|bid| led.latest_state(bid))
                        .map(|r| r.event.clone());
                    let m = if blocker.is_refused() {
                        msg::tier_blocked_refused(
                            &rel,
                            &sec.heading_text,
                            cand_tier,
                            open_n,
                            &blocker.heading_text,
                        )
                    } else if blocker_state.as_deref() == Some("asked") {
                        let bhash = record::section_hash(&rf.lines, blocker);
                        msg::tier_blocked_waiting(
                            &rel,
                            &sec.heading_text,
                            cand_tier,
                            open_n,
                            &blocker.heading_text,
                            blocker.id.as_deref().unwrap_or("<id>"),
                            &bhash[..record::VERSION_LEN],
                        )
                    } else {
                        msg::tier_blocked_unasked(
                            &rel,
                            &sec.heading_text,
                            cand_tier,
                            open_n,
                            &blocker.heading_text,
                        )
                    };
                    eprintln!("ask: {}", m);
                    return 1;
                }
            }
        }

        // Moot refusal: this candidate's own entry in `moot_pairs`, if any.
        if let Some((_, t, set_str, c)) = moot_pairs.iter().find(|(h, ..)| h == &sec.heading_text) {
            eprintln!(
                "ask: {}",
                msg::moot_candidate(&rel, &sec.heading_text, t, set_str, *c)
            );
            return 1;
        }

        // Per "Does the tool stop loaded phrases in a question?": a
        // provenance, code-state, or date tell anywhere in the
        // candidate's own body (heading through its hash range) refuses,
        // from the same scan `approve` runs on headings and context prose.
        {
            let (s, e) = record::hash_range(&rf.lines, sec);
            if let Some((line, phrase, what)) =
                tells::prose_tells_in(&rf.lines, s, e).into_iter().next()
            {
                eprintln!("ask: {}", msg::tell(&rel, line, &sec.heading_text, &phrase, what));
                return 1;
            }
        }

        let text = state::section_text_excluding_terms_line(&rf.lines, sec);

        // Refusal 5: a banned word in the candidate's own text, from the
        // single hash-based ban list scanner. `ask` stops at the first hit.
        if let Some(hit) = banned::hits(&text, &banned_entries, &terms_list).into_iter().next() {
            let suffix = msg::term_suffix(hit.term.as_deref());
            eprintln!(
                "ask: {}",
                msg::banned_word_in_candidate(&rel, &sec.heading_text, &suffix)
            );
            return 1;
        }

        // `terms::resolve` never matches a retired term (per "Does a retired term keep its words?"), so a
        // retired-match refusal cannot fire here; a word that only a
        // retired term owns simply pairs nothing, caught by refusal 6
        // below.
        let pairs = terms::resolve(&text, &terms_list);

        // Refusal 6: zero pairings.
        if pairs.is_empty() {
            eprintln!("ask: {}", msg::pairs_no_term(&rel, &sec.heading_text));
            return 1;
        }
    }

    // Per "Does a reference in a topic file carry the identifier of what
    // it names?": ask writes no ids. Every non-skeleton section already has one
    // here -- refusal 4 above (`plan_approved`) already refused whenever
    // one is missing, since `approve` mints every heading's id in the same
    // pass that makes the plan hash match, and any further edit that added
    // a heading lacking one would already have moved the plan hash.

    // Write/replace the `terms:` line directly under the `id:` line for
    // every candidate, bottom-to-top so earlier sections' line numbers are
    // unaffected by a section growing or shrinking by one line. Every
    // candidate reaching this point paired at least one term (refusal 6
    // above), so the pairings are never empty here: ask never deletes a
    // terms line, and a candidate section whose id line cannot be found is
    // an error, not a silent skip.
    let mut terms_touched = false;
    for i in (0..rf.sections.len()).rev() {
        if !candidate[i] {
            continue;
        }
        let sec = rf.sections[i].clone();
        let text = state::section_text_excluding_terms_line(&rf.lines, &sec);
        let pairs = terms::resolve(&text, &terms_list);
        let new_line = match terms::format_terms_line(&pairs) {
            Some(nl) => nl,
            None => {
                // Unreachable given refusal 6 above, but never silently
                // drop a binding if it somehow happens.
                eprintln!("ask: {}", msg::pairs_no_term_at_write(&rel, &sec.heading_text));
                return 1;
            }
        };
        match sec.terms_line {
            Some(tl) => {
                if rf.lines[tl] != new_line {
                    rf.lines[tl] = new_line;
                    terms_touched = true;
                }
            }
            None => match find_prefixed_line(&rf.lines, sec.start, sec.end, "id: ") {
                Some(idx) => {
                    rf.lines.insert(idx + 1, new_line);
                    terms_touched = true;
                }
                None => {
                    eprintln!("ask: {}", msg::no_id_line(&rel, &sec.heading_text));
                    return 1;
                }
            },
        }
    }
    if terms_touched {
        rf.sections = record::parse_sections(&rf.lines);
    }

    let mut new_rows = Vec::new();
    let mut summary: Vec<String> = Vec::new();
    for i in 0..rf.sections.len() {
        let sec = &rf.sections[i];
        // Per "Does a reference in a topic file carry the identifier of
        // what it names?": `approve` now gives a skeleton an id too, so an
        // id alone no longer means "this is a candidate" -- a skeleton
        // must still never be asked, exactly as `candidate` above already
        // excludes it from becoming one.
        if sec.is_skeleton {
            continue;
        }
        let Some(id) = sec.id.clone() else { continue };
        let cur = record::section_hash(&rf.lines, sec);
        let label = match led.latest_state(&id) {
            None => Some("asked"),
            Some(row) if row.event == "asked" && row.hash != cur => Some("re-asked"),
            _ => None,
        };
        let Some(label) = label else { continue };
        let tier = rf.tier_of(i).unwrap_or(1);
        new_rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "asked".to_string(),
            id: id.clone(),
            hash: cur,
            detail: sec.heading_text.clone(),
            version: crate::version::VERSION.to_string(),
        });
        summary.push(format!("{}: \"{}\" (tier {})", label, sec.heading_text, tier));

        // One `assumes` row per dependent (per "Does a depends entry
        // carry the answers it holds under?"): what set the
        // dependent currently declares on this question, frozen at ask
        // time so `check`'s `answers.rs` can later catch it moving.
        for (di, set) in rf.dependents_of(i) {
            let Some(set) = set else {
                // Guaranteed not to happen: the sets-mandatory gate above
                // already refused before any row was written if a
                // dependent in this tier carried no set. Skip rather than
                // panic if it somehow does.
                continue;
            };
            let dependent = &rf.sections[di];
            // Per "Does a reference in a topic file carry the identifier
            // of what it names?": an `assumes` row keys on the dependent's own
            // heading id, not its heading text -- every heading already has
            // one by the time `ask` runs (approve mints them all up front),
            // so the row stays valid across a rename that doesn't otherwise
            // change the plan.
            let Some(dep_id) = dependent.id.clone() else {
                continue;
            };
            let set_str = record::canonical_set(set);
            new_rows.push(ledger::Row {
                time: ledger::now_rfc3339(),
                event: "assumes".to_string(),
                id: id.clone(),
                hash: record::assumes_hash(&dep_id, set),
                detail: dep_id,
                version: crate::version::VERSION.to_string(),
            });
            summary.push(format!(
                "  assumes: \"{}\"={}",
                dependent.heading_text, set_str
            ));
        }
    }

    if new_rows.is_empty() {
        println!("ask: {}", msg::nothing_to_ask(&record_path.display().to_string()));
        return 0;
    }

    if let Err(e) = record::save(&rf) {
        eprintln!(
            "ask: {}",
            msg::write_failed(
                &record_path.display().to_string(),
                e,
                Next::agent("checks the error and retries"),
            )
        );
        return 1;
    }
    if let Err(e) = ledger::append(&ledger_path, &new_rows) {
        eprintln!(
            "ask: {}",
            msg::update_failed("ledger", e, Next::agent("checks the error and retries"))
        );
        return 1;
    }

    for s in &summary {
        println!("{}", s);
    }
    0
}

/// The line index of the first line in `[start+1, end)` that starts with
/// `prefix`. Used to find the `id: ` line to write the `terms: ` line
/// directly under.
fn find_prefixed_line(lines: &[String], start: usize, end: usize, prefix: &str) -> Option<usize> {
    (start + 1..end).find(|&j| lines[j].starts_with(prefix))
}
