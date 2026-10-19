use crate::actions;
use crate::depends;
use crate::ledger;
use crate::msg::{self, Actor};
use crate::out::{self, Line};
use crate::record;
use crate::repo;
use crate::rules;
use crate::rules::{Class, Command, Finding};
use crate::state;
use crate::status;
use crate::terms;
use crate::view;
use std::path::{Path, PathBuf};

pub fn run(given: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint_stop(
                Command::Ask,
                &msg::not_a_repo("ask", &[&given.display().to_string()], Actor::Agent),
            );
            return 1;
        }
    };
    if super::refused_in_satellite(
        &root,
        Command::Ask,
        &[&given.display().to_string()],
        Actor::Agent,
    ) {
        return 1;
    }
    let ledger_path = root.join(repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    // The path as the shell that ran ask reads it, from the repository
    // root: every message names the file by it.
    let rel = match state::given_rel(&root, &given) {
        Some(rel) if crate::rules::is_topic_file_path(&rel) => rel,
        rel => {
            out::eprint_stop(
                Command::Ask,
                &msg::plan_not_a_topic_file(&rel.unwrap_or_else(|| given.display().to_string())),
            );
            return 1;
        }
    };
    let record_path = root.join(&rel);
    // Asking a topic file that loads makes the session a planner (6c406624),
    // whatever the run finds after (the terms refusals below included). A
    // file that is missing or unreadable marks nothing; it is reported after
    // the terms refusals.
    let loaded = record::load(&record_path);
    if loaded.is_ok() {
        if let Err(e) = crate::session::mark_planner(&root) {
            out::eprint_stop(
                Command::Ask,
                &msg::write_failed(
                    repo::ROLES_PATH,
                    &e,
                    msg::Next::owner(msg::Act::Rewrites(repo::ROLES_PATH.to_string())),
                ),
            );
            return 1;
        }
    }

    // Refusals 1-3: adrdom/terms.md missing, unreadable/shape errors, or
    // not approved. These come before anything about the topic file itself is
    // even loaded, since none of them depend on it. Shared with `approve`
    // (`actions::require_terms`).
    let tf = match actions::require_terms(&root, &led, &rel, Command::Ask) {
        Ok(tf) => tf,
        Err(e) => {
            out::eprint(&e);
            return 1;
        }
    };
    let terms_list: Vec<terms::Term> = tf.terms.clone();

    let mut rf = match loaded {
        Ok(mut rf) => {
            // `record::load` leaves the path as given; the ledger names the
            // file by its path from the repository root.
            rf.rel = rel.clone();
            rf
        }
        Err(e) => {
            out::eprint_stop(
                Command::Ask,
                &msg::read_failed(&rel, &e, msg::topic_file_unread("ask", &rel, &e)),
            );
            return 1;
        }
    };

    // Per "Does check fail a question answered with two different
    // options?" (e3200081-9995-4bbe-983f-2f430dc341e7, rule 5): a question
    // in conflict takes priority over everything else this run -- whatever
    // tier it is in, whether or not the file has other questions to ask.
    // Handled and reported on its own, ahead of every other gate below
    // (shape, plan/context approval, tier cap): none of that bears on
    // re-asking a question whose text never moved. When the file has other
    // questions to ask, only the conflict is asked this run; a further
    // `ask` run asks the rest.
    if let Some((idx, id)) = find_conflicted_section(&rf, &led) {
        return do_conflict_reask(&root, &record_path, rf, &led, idx, &id);
    }

    // Shape and parse errors refuse first, before anything else is even
    // considered: the same "parse" category `approve`/`check` refuse on
    // (an option with no decision sentence or more than one, a duplicate id
    // line, an answer naming a missing option -- used to be a parse error
    // only `check` read; `record_refusals` is now the one shared validation
    // all three commands read it through).
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    // Every current decision's heading text, repo-wide: built once here,
    // the same map `approve`/`check` build, and passed to
    // `record_refusals`. `ask` has nothing else loaded repo-wide (only
    // `rf`, the one file it was asked about), so this is the one caller
    // that still has to load every topic file for it.
    let all_records: Vec<record::RecordFile> = record::load_all(&root);
    let cross_file_headings = depends::cross_file_heading_index(&all_records, &led);

    // Plan/context approval state is read off `rf` exactly as loaded,
    // via `state::plan_approved_hashes`/`context_approved_hash`.
    let raw_question_hashes = record::question_hashes(&rf);
    let raw_context_hash = record::context_hash(&rf);

    // Every heading by its identifier, any state: a refusal names the
    // question beside an identifier.
    let names = crate::view::heading_names(&all_records, &led);
    // `all_records` is still as loaded, so its plan hashes are the raw ones.
    let replaced_base = depends::replaced_base(
        &all_records,
        &depends::approved_plan_rels(&all_records, &led),
        &cross_file_headings,
        &led,
    );
    let replaced = depends::replaced_ids(&replaced_base, &rf, &cross_file_headings, &led);
    // The blame files the `blame:` lines are judged against, off the working
    // tree like the topic files.
    let (blame_index, _) = crate::blame::Index::read(&root, &repo::Source::WorkingTree, &led);
    // Every rule the file breaks, judged once (`rules::record_refusals`, the
    // judgement `approve` and `check` read): ask refuses on the parse
    // findings first, then on what is found in the questions it asks.
    let mut findings = rules::record_refusals(
        &rel,
        &rf,
        &terms_list,
        &phrase_lookup,
        &cross_file_headings,
        &names,
        &all_records,
        &replaced,
        &blame_index,
        &led,
    );
    // A banned word in a question's text, the judgement show, plan and
    // approve read too (`rules::banned_in_questions`).
    findings.extend(rules::banned_in_questions(
        &root,
        &rel,
        &rf,
        &terms_list,
        &led,
    ));
    // Each printed as the finding it is, under its own rule, as `show` and
    // `check` print it.
    let shape: Vec<out::Line> = findings
        .iter()
        .filter(|r| r.class == Class::Parse)
        .cloned()
        .map(out::Line::Finding)
        .collect();
    if !shape.is_empty() {
        out::eprint(&shape);
        return 1;
    }

    // Refuse (and change nothing) on the rules on each question's answer or
    // refusal (`rules::closed_findings`, in the judgement above): a closed
    // question edited since, a line no row backs, two standing answers. The
    // findings and moves `check` fails the file on, named the same way.
    let closed: Vec<out::Line> = findings
        .iter()
        .filter(|f| f.class == Class::Answers)
        .cloned()
        .map(out::Line::Finding)
        .collect();
    if !closed.is_empty() {
        out::eprint(&closed);
        return 1;
    }

    // Refusal 4: plan / context must be approved before ask will touch the
    // file at all -- checked against the hashes captured above.
    // The refusal names the moves of the file, the ones `show` names for it.
    let file_moves = || -> crate::moves::Report {
        let tests = crate::commands::check::test_index(&root, &repo::Source::WorkingTree);
        let mut report = match all_records.iter().find(|f| f.rel == rel) {
            Some(loaded) => crate::moves::of_topic_file_in(
                &root,
                loaded,
                &all_records,
                &terms_list,
                &led,
                &tests,
            ),
            None => crate::moves::Report::default(),
        };
        if report.findings.is_empty() && report.moves.is_empty() {
            report
                .moves
                .push(crate::moves::Move::new(crate::moves::MoveKind::Approve {
                    path: rel.clone(),
                }));
        }
        report
    };
    if !state::plan_approved_hashes(&led, &rel, &raw_question_hashes) {
        let status = crate::status::plan_status(&led, &rel, false);
        let report = file_moves();
        let mut lines = vec![Line::Finding(Finding::said(
            Class::Plan,
            msg::plan_not_approved(&rel, status, &report),
        ))];
        lines.extend(report.findings.into_iter().map(Line::Finding));
        out::eprint(&lines);
        return 1;
    }
    if !state::context_approved_hash(&led, &rel, &raw_context_hash) {
        let status = crate::status::prose_status(&led, &rel, false);
        let report = file_moves();
        let mut lines = vec![Line::Finding(Finding::said(
            Class::Plan,
            msg::context_not_approved(&rel, status, &report),
        ))];
        lines.extend(report.findings.into_iter().map(Line::Finding));
        out::eprint(&lines);
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
        .map(|sec| status::open_to_ask(&led, &rf, sec))
        .collect();

    // Derive-error refusal (8.4/8.7): a derivation failure means `approve`
    // would also refuse on this same file, so the fix has to happen before
    // either command can move: point the owner at the topic file first, the
    // re-approval second.
    if let Some(f) = rules::derive_finding(&rel, &rf, &cross_file_headings, &led) {
        out::eprint(&[Line::Finding(f)]);
        return 1;
    }

    // Cap refusal (per "Does the tool cap the questions in a tier?"): any
    // derived tier over `TIER_CAP` live questions,
    // file-wide (mirrors approve's cap check, prefixed `ask:`). Dead
    // (refused/superseded/withdrawn) questions don't count.
    if let Some(f) = rules::cap_finding(&rel, &rf, &replaced, &led) {
        out::eprint(&[Line::Finding(f)]);
        return 1;
    }

    // Per "What does ask put to the owner at one run?": every candidate is
    // asked, whatever its tier. What `answer` takes first is not ask's
    // to order: `answer` refuses a question that waits (`state::answer_waits`)
    // and `next` names who moves when.

    // From here on every refusal is about one question and reads the same
    // file and ledger as the others, so none of them needs another to have
    // passed. They are gathered and printed together, and nothing is
    // written while there is one: a question with three faults is told all
    // three in one run.
    let mut refusals: Vec<Finding> = Vec::new();

    // Answer sets (per "Does a depends entry carry the answers it holds
    // under?"): before asking anything, every set on a heading dep must name
    // only options its target has (`rules::letter_refusals`, the judgement
    // `approve` reads), only when there is something new to ask this run.
    if candidate.iter().any(|c| *c) {
        refusals.extend(rules::letter_refusals(&rel, &rf, &led));
    }

    for (i, _sec) in rf.sections.iter().enumerate() {
        if !candidate[i] {
            continue;
        }

        // The supersede rules (`rules::supersedes_refusals`, the one place
        // `approve`, `check` and `answer` read too): the line freezes at ask,
        // so a candidate naming a target that is not current, leaving out a
        // decision standing on one, or standing on one refuses here.
        refusals.extend(rules::supersedes_refusals(
            &rel,
            &rf,
            &cross_file_headings,
            &names,
            &all_records,
            &led,
            Some(i),
        ));

        // Every rule the question breaks (`rules::record_refusals`, the
        // judgement `approve` and `check` read): a loaded phrase, a line
        // over its word limit, a question that pairs no term (per "Does the
        // tool stop loaded phrases in a question?", "Does ask refuse an
        // option over a word limit?", "Does ask refuse a question with no
        // term?"), and what its depends line stands on.
        for f in findings.iter().filter(|f| f.sections.contains(&i)) {
            if !refusals.iter().any(|r| r.text() == f.text()) {
                refusals.push(f.clone());
            }
        }
    }

    if !refusals.is_empty() {
        out::eprint(&refusals.into_iter().map(Line::Finding).collect::<Vec<_>>());
        return 1;
    }

    // Per "What does a reference in a topic file carry?": ask writes no ids. Every non-skeleton
    // section already has one here -- refusal 4 above (`plan_approved`) already refused whenever
    // one is missing, since `approve` mints every heading's id in the same pass that makes the plan
    // hash match, and any further edit that added a heading lacking one would already have moved
    // the plan hash.

    // Write/replace the `terms:` line directly under the `id:` line for
    // every candidate, bottom-to-top so earlier sections' line numbers are
    // unaffected by a section growing or shrinking by one line. Every
    // candidate reaching this point paired at least one term (refusal 6
    // above), so the pairings are never empty here: ask never deletes a
    // terms line.
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
                out::eprint(&[Line::Finding(Finding::said(
                    Class::Terms,
                    msg::pairs_no_term_at_write(&rel, &sec.heading_text),
                ))]);
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
            None => {
                // Approve gives every question its id line.
                if let Some(idx) = find_prefixed_line(&rf.lines, sec.start, sec.end, "id: ") {
                    rf.lines.insert(idx + 1, new_line);
                    terms_touched = true;
                }
            }
        }
    }
    if terms_touched {
        rf.sections = record::parse_sections(&rf.lines);
    }

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A): the "asked" row and its "assumes" rows below all share it.
    let write_time = ledger::write_time(&led);
    let mut new_rows = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        // Per "What does a reference in a topic file carry?": `approve` now gives a skeleton an id
        // too, so an id alone no longer means "this is a candidate" -- a skeleton must still never
        // be asked, exactly as `candidate` above already excludes it from becoming one.
        if sec.is_skeleton || !candidate[i] {
            continue;
        }
        let Some(id) = sec.id.clone() else { continue };
        // Read again on the text as rewritten (its terms line may be new):
        // the one test of a question open to ask.
        if !status::open_to_ask(&led, &rf, sec) {
            continue;
        }
        let cur = record::section_hash(&rf.lines, sec);
        new_rows.push(ledger::Row {
            time: write_time.clone(),
            event: "asked".to_string(),
            id: id.clone(),
            hash: cur,
            detail: sec.heading_text.clone(),
            version: crate::version::VERSION.to_string(),
        });

        // One `assumes` row per dependent (per "Does a depends entry
        // carry the answers it holds under?"): what set the
        // dependent currently declares on this question, frozen at ask
        // time so `check`'s `answers.rs` can later catch it moving.
        for (di, set) in rf.dependents_of(i) {
            let Some(set) = set else {
                // Approve refuses a dependent in the file with no set on a
                // question not yet answered, so none is left here. Skip
                // rather than panic if one is.
                continue;
            };
            let dependent = &rf.sections[di];
            // Per "What does a reference in a topic file carry?": an `assumes` row keys on the
            // dependent's own heading id, not its heading text -- every heading already has one by
            // the time `ask` runs (approve mints them all up front), so the row stays valid across
            // a rename that doesn't otherwise change the plan.
            let Some(dep_id) = dependent.id.clone() else {
                continue;
            };
            new_rows.push(ledger::Row {
                time: write_time.clone(),
                event: "assumes".to_string(),
                id: id.clone(),
                hash: record::assumes_hash(&dep_id, set),
                detail: dep_id,
                version: crate::version::VERSION.to_string(),
            });
        }
    }

    let again = || msg::after_fix_agent_runs(crate::moves::command(&["adr", "ask", &*rel]));
    // The file as it stands on disk, put back when the ledger refuses the
    // rows: never a file rewritten for rows no ledger holds.
    let original = std::fs::read_to_string(&record_path).ok();
    let rewritten = original.as_deref() != Some(record::join_lines(&rf.lines).as_str());
    // Nothing to ask writes nothing and prints nothing: what is left to do
    // is the move `show` names (per "What does a command that writes
    // print?", 069a6b4a A). A terms line written again in place (one
    // edited or deleted by hand under an asked question) is a file written
    // with no row to cover it: saved, and named by one line.
    if new_rows.is_empty() {
        if !rewritten {
            return 0;
        }
        if let Err(e) = record::save(&rf) {
            out::eprint_stop(Command::Ask, &msg::write_failed(&rel, e, again()));
            return 1;
        }
        out::print_events(&[out::Event::FileWritten { path: rel.clone() }]);
        return 0;
    }

    if let Err(e) = record::save(&rf) {
        out::eprint_stop(Command::Ask, &msg::write_failed(&rel, e, again()));
        return 1;
    }
    if let Err(e) = ledger::append(&ledger_path, &new_rows) {
        if let Some(original) = &original {
            let _ = std::fs::write(&record_path, original);
        }
        out::eprint_stop(Command::Ask, &msg::update_failed("ledger", e, again()));
        return 1;
    }

    out::print_events(
        &actions::Outcome::of_rows(&new_rows, &view::heading_texts(std::slice::from_ref(&rf)))
            .events,
    );
    0
}

/// The line index of the first line in `[start+1, end)` that starts with
/// `prefix`. Used to find the `id: ` line to write the `terms: ` line
/// directly under.
fn find_prefixed_line(lines: &[String], start: usize, end: usize, prefix: &str) -> Option<usize> {
    (start + 1..end).find(|&j| lines[j].starts_with(prefix))
}

/// The first section in `rf` that is in conflict (per
/// "Does check fail a question answered with two different options?",
/// e3200081-9995-4bbe-983f-2f430dc341e7): it carries an id, its
/// `**Answer:**` line is present (a genuine answer or refusal to remove --
/// nothing to re-ask without one), and `state::conflicting_rulings` finds
/// two or more standing rulings for its id. Returns the section's own
/// index and its id, for `do_conflict_reask`.
fn find_conflicted_section(
    rf: &record::RecordFile,
    led: &ledger::Ledger,
) -> Option<(usize, String)> {
    for (i, sec) in rf.sections.iter().enumerate() {
        let Some(id) = &sec.id else { continue };
        if sec.answer_line.is_none() {
            continue;
        }
        if state::conflicting_rulings(led, id).is_some() {
            return Some((i, id.clone()));
        }
    }
    None
}

/// Removes the `**Answer:**` line `finish_answer` (src/actions) once
/// spliced in for section `idx`, and the blank line it inserted directly
/// above it, leaving the section exactly as it looked before it was first
/// answered: `finish_answer` always inserts exactly those two lines
/// (`[String::new(), "**Answer:** ..."]`) at the section's hash end, so
/// removing them is the exact reverse of that splice, whatever the answer
/// text was. Reparses `rf.sections` afterward (the same lightweight
/// re-derive `ask`'s own terms-line rewrite already uses below: section
/// count and order are unaffected by removing two lines from inside one
/// section, so `rf.tiers` -- not recomputed here -- stays index-aligned).
fn strip_answer_line(rf: &mut record::RecordFile, idx: usize) {
    let sec = &rf.sections[idx];
    let al = sec
        .answer_line
        .expect("find_conflicted_section only returns a section with an answer line");
    let start = sec.start;
    let remove_from = if al > start && rf.lines[al - 1].trim().is_empty() {
        al - 1
    } else {
        al
    };
    rf.lines.drain(remove_from..=al);
    rf.sections = record::parse_sections(&rf.lines);
}

/// Asks a conflicted question again (rule 5): strips its `**Answer:**`
/// line, saves the topic file, then writes a fresh `asked` row (and its
/// `assumes` rows) exactly as an ordinary ask of that question would --
/// the section's text never moved, so its hash is the same one the
/// question was first asked under. Bypasses every other gate `run` applies
/// to an ordinary candidate (plan/context approval, the tier cap, the open
/// tier, banned/terms checks): none of them bear on re-asking text
/// that hasn't changed.
fn do_conflict_reask(
    root: &Path,
    record_path: &Path,
    mut rf: record::RecordFile,
    led: &ledger::Ledger,
    idx: usize,
    id: &str,
) -> i32 {
    let heading = rf.sections[idx].heading_text.clone();
    // The file as it stands, put back when the ledger refuses the row: the
    // answer line it strips is what finds the conflict again, so the move
    // the refusal names (`adr ask` again) finds it.
    let original = std::fs::read_to_string(record_path).ok();

    strip_answer_line(&mut rf, idx);

    if let Err(e) = record::save(&rf) {
        out::eprint_stop(
            Command::Ask,
            &msg::write_failed(
                &state::rel_path(root, record_path),
                e,
                msg::after_fix_agent_runs(crate::moves::command(&[
                    "adr",
                    "ask",
                    &state::rel_path(root, record_path),
                ])),
            ),
        );
        return 1;
    }

    let write_time = ledger::write_time(led);
    let sec = &rf.sections[idx];
    let cur = record::section_hash(&rf.lines, sec);

    let mut new_rows = vec![ledger::Row {
        time: write_time.clone(),
        event: "asked".to_string(),
        id: id.to_string(),
        hash: cur,
        detail: heading.clone(),
        version: crate::version::VERSION.to_string(),
    }];

    // One `assumes` row per dependent that already carries a set, exactly
    // as an ordinary ask writes them (a dependent with no set at all is
    // skipped rather than refused).
    for (di, set) in rf.dependents_of(idx) {
        let Some(set) = set else { continue };
        let dependent = &rf.sections[di];
        let Some(dep_id) = dependent.id.clone() else {
            continue;
        };
        new_rows.push(ledger::Row {
            time: write_time.clone(),
            event: "assumes".to_string(),
            id: id.to_string(),
            hash: record::assumes_hash(&dep_id, set),
            detail: dep_id,
            version: crate::version::VERSION.to_string(),
        });
    }

    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    if let Err(e) = ledger::append(&ledger_path, &new_rows) {
        if let Some(original) = &original {
            let _ = std::fs::write(record_path, original);
        }
        out::eprint_stop(
            Command::Ask,
            &msg::update_failed(
                "ledger",
                e,
                msg::after_fix_agent_runs(crate::moves::command(&[
                    "adr",
                    "ask",
                    &state::rel_path(root, record_path),
                ])),
            ),
        );
        return 1;
    }

    out::print_events(
        &actions::Outcome::of_rows(&new_rows, &view::heading_texts(std::slice::from_ref(&rf)))
            .events,
    );
    0
}
