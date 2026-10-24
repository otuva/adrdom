//! Approve of a topic file's headings and prose.
use super::approve::{insert_missing_ids, rewrite_dep_entry};
use super::{findings_of, require_terms, stop, Outcome, Stops};
use crate::blame;
use crate::depends;
use crate::ledger::{self, Ledger};
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rules::{self, removal_holds, Command};
use crate::state;
use crate::terms;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A topic file: parse, refuse on shape errors; resolve every `depends:`
/// entry (per "What does a reference in a topic file carry?": the file holds ids, the agent writes
/// words, and `approve` is what turns one into the other); check every ledger id that now appears
/// in no topic file, refusing on a current (answered) one, before anything is written --
/// withdrawing an asked/refused one and allowing a superseded one silently happen after, since
/// neither is a refusal; only then mint an id for every heading that lacks one, compute plan hash
/// and context hash over the rewritten file, write an `approved` row if it differs from the latest
/// for the path, a `context` row likewise, then ask every question open to ask (its `terms:` line,
/// its `asked` and `assumes` rows), and finally write the withdrawn rows, all in one append. A
/// question in conflict is asked again first and alone. Nothing is written to disk (file or
/// ledger) when any check refuses.
pub(super) fn do_approve_record(
    root: &Path,
    file: &Path,
    rel: &str,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, Stops> {
    let mut rf = record::load(file).map_err(|e| {
        stop(
            Command::Approve,
            msg::read_failed(
                rel,
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["approve", &*rel])),
            ),
        )
    })?;
    // The file as the repository names it, whatever path it was loaded by:
    // every rule that compares a ledger row's path with `rf.rel` reads the
    // same path for it whichever spelling the command was given.
    rf.rel = rel.to_string();

    // Refuse up front, before a single `depends:` entry is resolved, when
    // adrdom/terms.md is missing, shape-broken, or not yet approved: the
    // same gate `approve`/`answer`/`refuse` apply via `require_terms`. Without
    // this, a real term with no id minted yet (an unapproved terms.md mints
    // nothing to disk) silently fails to resolve and every depends entry
    // naming it refuses "which is no term" instead of the one clear "terms
    // not approved" line.
    let tf = require_terms(root, led, rel, Command::Approve)?;
    // A question in conflict is asked again first and alone (per "Does
    // check fail a question answered with two different options?",
    // e3200081 A): none of the gates below bears on text that never moved.
    if let Some((idx, id)) = super::ask::conflicted_section(&rf, led) {
        return super::ask::reask_conflict(root, rf, led, idx, &id);
    }
    // Read off the file as written.
    let planning = rules::planning_rules_apply(led, &rf, rel);
    let terms_list: &[terms::Term] = &tf.terms;
    // Built once per command run (not per depends entry): `resolve_dep`
    // used to call `terms::resolve_word`, which rebuilt this same map from
    // scratch on every entry.
    let phrase_lookup = terms::phrase_lookup(terms_list);

    // Every topic file, loaded once here rather than twice: this used to be
    // loaded again, further down, only for `missing_answered_refusals` --
    // after `cross_file_heading_index` had already loaded every file
    // itself to build its own copy of the same thing. Loaded via
    // `record::load_all` (root-relative `.rel`, matching what `cross_file_heading_index`
    // always printed) straight off disk, still unwritten at this point, so
    // its ids are exactly what's on disk; minting new ids for skeleton
    // headings below never removes an id already in the ledger's
    // `state_ids`, so loading before that rewrite changes nothing either
    // read below depends on.
    let all_records: Vec<RecordFile> = record::load_all(root);
    // Every current decision's heading text, repo-wide (the index a
    // `supersedes:` entry is resolved against): built once here, from
    // `all_records` above.
    let cross_file_headings = depends::cross_file_heading_index(&all_records, led);

    // Every refusal this file can earn -- shape, tells, a missing
    // depends line, depends resolution (including a frozen section's
    // edit and the derived-tiers error), and the stand cap -- collected
    // here (`record_refusals`, shared with `check`) instead of returned at
    // the first, so one `approve` call tells the agent everything it owes
    // the owner before asking again.
    let names = crate::view::heading_names(&all_records, led);
    // `all_records` is still as loaded, so its plan hashes are the raw ones.
    let replaced_base = depends::replaced_base(
        &all_records,
        &depends::approved_plan_rels(&all_records, led),
        &cross_file_headings,
        led,
    );
    let replaced = depends::replaced_ids(&replaced_base, &rf, &cross_file_headings, led);
    // Every blame file, read off the working tree like the topic files: the
    // `blame:` lines are judged against it and turned to identifiers by it.
    let (blame_index, _) = blame::Index::read(root, &repo::Source::WorkingTree, led);
    // Every rule this file breaks, judged once (`rules::judge_topic_file`,
    // the judgement `check` reads too), and the rules on the records as a
    // whole (`rules::judge_records`): collected, not returned at the first,
    // so one `approve` call tells the agent everything it owes the owner
    // before asking again. Nothing is written while one stands.
    let judge = rules::TopicFileJudge {
        root,
        rel,
        rf: &rf,
        all: &all_records,
        terms_list,
        phrase_lookup: &phrase_lookup,
        cross_file_headings: &cross_file_headings,
        names: &names,
        replaced: &replaced,
        blame: &blame_index,
        led,
        planning,
        shared: None,
        directory: true,
    };
    let refusals = findings_of(
        rules::judge_topic_file(&judge)
            .into_iter()
            .chain(rules::judge_records(&all_records, led))
            .collect(),
    );

    // Per "Does approve turn the supersedes line into identifiers?" (B):
    // each entry of a `supersedes:` line is the heading of a current
    // decision (in any topic file) or its id, and becomes the id, the way
    // a `depends:` entry does. A section that is already frozen keeps its
    // line as it is: its target is superseded by now, and the section's
    // own hash guards the line.
    let mut supersedes_ids: HashMap<usize, Vec<String>> = HashMap::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        // Once asked, the line is frozen with the body (per "Does one
        // decision supersede several?"): a split sibling's target may be
        // superseded by now, and approve leaves the line as it is.
        // An asked section whose text no longer stands as asked is being
        // edited to be asked again: its line is turned like an unasked one's.
        let stands_as_asked = crate::view::asked_text_stands(led, &rf, sec);
        if sec.supersedes.is_empty() || stands_as_asked || state::is_frozen(led, sec) {
            continue;
        }
        // A target that names no decision (or several) is `record_refusals`'
        // refusal above; only the ones that resolve are turned.
        let ids: Vec<String> =
            depends::supersedes_entries(&sec.supersedes, &cross_file_headings, led)
                .iter()
                .filter_map(|old| depends::resolve_supersedes(old, &cross_file_headings, led).ok())
                .collect();
        supersedes_ids.insert(i, ids);
    }

    let present_ids: HashSet<String> = rules::held_ids(&all_records)
        .into_iter()
        .map(str::to_string)
        .collect();
    let state_ids = led.state_ids();

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A), taken once here from `led` (loaded before this run made any
    // change) and reused below for both the withdrawn rows and, further
    // down, the approved/context rows -- two separate `append` calls this
    // one run can make, both carrying this same time.
    let write_time = ledger::write_time(led);

    // Always computed (harmless when `refusals` is already non-empty from
    // an earlier category): only acted on -- ledger rows appended -- once
    // every category above has been confirmed clean, below.
    let mut withdrawn_rows: Vec<ledger::Row> = Vec::new();
    // An id of a removed file awaiting its removal's approval is settled by
    // that approval (per "What may leave a topic file by a plan change?"),
    // never by the approval of an unrelated file.
    let removed_held: HashSet<String> =
        rules::removal_awaiting(led, &|rel| root.join(rel).exists())
            .iter()
            .flat_map(|rel| removal_holds(root, rel, led, &present_ids).held)
            .map(|(id, _)| id)
            .collect();
    for id in &state_ids {
        if present_ids.contains(id) || removed_held.contains(id) {
            continue;
        }
        // A refused id absent from every file is withdrawn exactly like a
        // struck asked one (per "Does approve withdraw an asked question
        // struck from the plan?"): the refusal already closed it with no
        // decision, so there's nothing left to protect. An answered one is
        // handled above, by `missing_answered_refusals`.
        if let Some(r) = led.latest_state(id) {
            if r.event == "asked" || r.event == "refused" {
                withdrawn_rows.push(ledger::Row {
                    time: write_time.clone(),
                    event: "withdrawn".to_string(),
                    id: id.clone(),
                    hash: "-".to_string(),
                    detail: r.detail.clone(),
                    version: crate::version::VERSION.to_string(),
                });
            }
        }
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }

    // Every check above passed: mint an id for every heading that lacks one
    // (skeletons included), reparse so every heading's id is available, then
    // rewrite each section's single `depends:` line to one line of ids
    // (per "Does a reference line hold its whole list on one line?"), entry
    // by entry, keeping any `=set` suffix.
    let work_lines = insert_missing_ids(&rf, rf.lines.clone())?;
    let mut work_rf = record::build(rf.path.clone(), rel.to_string(), work_lines);

    let sections_snapshot = work_rf.sections.clone();
    for sec in &sections_snapshot {
        let Some(line_idx) =
            (sec.start + 1..sec.end).find(|&j| work_rf.lines[j].starts_with("depends: "))
        else {
            continue;
        };
        let parts: Vec<String> = sec
            .depends
            .iter()
            .map(|d| rewrite_dep_entry(d, sec, &work_rf, terms_list, &phrase_lookup, led))
            .collect();
        work_rf.lines[line_idx] = format!("depends: {}", parts.join(", "));
    }
    // The `supersedes:` line, kept where the writer put it (under `id:`
    // and `terms:` once asked), holds ids from here on.
    for (i, sec) in sections_snapshot.iter().enumerate() {
        let Some(ids) = supersedes_ids.get(&i) else {
            continue;
        };
        if let Some(j) =
            (sec.start + 1..sec.end).find(|&j| work_rf.lines[j].starts_with("supersedes: "))
        {
            work_rf.lines[j] = format!("supersedes: {}", ids.join(", "));
        }
    }
    // The `blame:` line holds identifiers too (per "What does a question
    // carry of the blame file it came from?"): each name becomes the
    // identifier on the blame file's id line, the ranges as written. Only a
    // question whose text can still change is turned; an asked question that
    // stands as asked, an answered one and the rest keep their line.
    for (i, sec) in sections_snapshot.iter().enumerate() {
        if sec.blame.is_empty() || !crate::status::open_to_ask_text(led, &rf, &rf.sections[i]) {
            continue;
        }
        let Some(j) = (sec.start + 1..sec.end).find(|&j| work_rf.lines[j].starts_with("blame: "))
        else {
            continue;
        };
        let entries: Vec<String> = sec
            .blame
            .iter()
            .map(|e| {
                let file = blame_index
                    .named(&e.file)
                    .and_then(|b| b.id())
                    .unwrap_or(&e.file);
                std::iter::once(file.to_string())
                    .chain(e.ranges.iter().map(|(from, to)| format!("{}-{}", from, to)))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        work_rf.lines[j] = format!("blame: {}", entries.join(", "));
    }
    // Reparse once more so the ask below and `plan_hash`/`context_hash` see
    // the file exactly as it will be written.
    let mut work_rf = record::build(work_rf.path.clone(), rel.to_string(), work_rf.lines.clone());

    // The plan accepted, approve asks every question of it open to ask (per
    // "Does approve ask the questions of the file?", 3762e14b A): its
    // `terms:` line written into the file, its rows written with the
    // approval's below.
    let asked_rows = super::ask::ask_open_questions(&mut work_rf, terms_list, led, &write_time)?;

    let plan_hash = record::plan_hash(&work_rf);
    let context_hash = record::context_hash(&work_rf);

    // Whether the save below changes the file's bytes (ids written in, a
    // reference line rewritten to identifiers, line ends made one kind).
    let original = std::fs::read_to_string(&work_rf.path).ok();
    let bytes_change = original.as_deref() != Some(record::join_lines(&work_rf.lines).as_str());
    record::save(&work_rf).map_err(|e| {
        stop(
            Command::Approve,
            msg::write_failed(
                &file.display().to_string(),
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["approve", &*rel])),
            ),
        )
    })?;

    let mut rows: Vec<ledger::Row> = Vec::new();

    // One row per question (per "Does approve hash each question on its
    // own?"): its id and its own hash, written when that differs from the
    // question's latest row for this file; then the file's row, whose hash
    // is derived from the sorted question hashes.
    let question_hashes = record::question_hashes(&work_rf);
    for (sec, qh) in work_rf.sections.iter().zip(&question_hashes) {
        let Some(id) = &sec.id else { continue };
        let unchanged = led
            .rows
            .iter()
            .rfind(|r| r.event == "approved" && &r.id == id)
            .is_some_and(|r| &r.hash == qh && r.detail == rel);
        if !unchanged {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: "approved".to_string(),
                id: id.clone(),
                hash: qh.clone(),
                detail: rel.to_string(),
                version: crate::version::VERSION.to_string(),
            });
        }
    }
    // A set approved while its in-file target is asked and not yet answered
    // stays after the answer (per "Does a depends entry on a question in its
    // file carry an answer set?"): the `assumes` row `approve` writes for an
    // entry, written here for an entry that came after the ask.
    for sec in &work_rf.sections {
        let Some(dep_id) = &sec.id else { continue };
        if led.latest_asked(dep_id).is_some() {
            continue;
        }
        for d in &sec.depends {
            let record::Dep::Id {
                id: tid,
                set: Some(set),
            } = d
            else {
                continue;
            };
            let waiting = work_rf.sections.iter().any(|t| t.id.as_ref() == Some(tid))
                && led.latest_state(tid).is_some_and(|r| r.event == "asked");
            let hash = record::assumes_hash(dep_id, set);
            let written = led
                .assumes_for(tid)
                .iter()
                .any(|r| &r.detail == dep_id && r.hash == hash);
            if waiting && !written {
                rows.push(ledger::Row {
                    time: write_time.clone(),
                    event: "assumes".to_string(),
                    id: tid.clone(),
                    hash,
                    detail: dep_id.clone(),
                    version: crate::version::VERSION.to_string(),
                });
            }
        }
    }
    let plan_changed = led
        .latest_for_path("approved", rel)
        .map(|r| r.hash.as_str())
        != Some(plan_hash.as_str());
    if plan_changed {
        rows.push(ledger::Row {
            time: write_time.clone(),
            event: "approved".to_string(),
            id: "-".to_string(),
            hash: plan_hash.clone(),
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        });
    }

    let context_changed =
        led.latest_for_path("context", rel).map(|r| r.hash.as_str()) != Some(context_hash.as_str());
    if context_changed {
        rows.push(ledger::Row {
            time: write_time.clone(),
            event: "context".to_string(),
            id: "-".to_string(),
            hash: context_hash.clone(),
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        });
    }

    // The rows of the ask, then the withdrawn rows, in one append with the
    // approval's: an `assumes` row approve wrote above for a target asked
    // again in this run is not written twice.
    for r in asked_rows {
        let twice = r.event == "assumes"
            && rows.iter().any(|w| {
                w.event == r.event && w.id == r.id && w.hash == r.hash && w.detail == r.detail
            });
        if !twice {
            rows.push(r);
        }
    }
    rows.extend(withdrawn_rows);
    // The file as it stood goes back when the ledger refuses the rows: never
    // a file rewritten for rows no ledger holds.
    if let Err(e) = ledger::append(ledger_path, &rows) {
        if bytes_change {
            if let Some(original) = &original {
                let _ = std::fs::write(&work_rf.path, original);
            }
        }
        return Err(stop(
            Command::Approve,
            msg::update_failed(
                crate::repo::LEDGER_PATH,
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["approve", rel])),
            ),
        ));
    }

    let mut outcome = Outcome::of_rows(
        &rows,
        &crate::view::heading_texts(std::slice::from_ref(&work_rf)),
    );
    // A file written that no row of this run covers is named by one line of
    // its own (per "What does a command that writes print?", 069a6b4a A).
    let covered = rows.iter().any(|r| {
        ((r.event == "approved" || r.event == "context") && r.detail == rel) || r.event == "asked"
    });
    if bytes_change && !covered {
        outcome.events.push(crate::out::Event::FileWritten {
            path: rel.to_string(),
        });
    }
    Ok(outcome)
}
